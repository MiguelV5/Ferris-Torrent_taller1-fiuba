use crate::torrent::user_interface::constants::MessageUI;
use crate::torrent::user_interface::ui_handler;
use crate::torrent::{
    client::{
        medatada_analyzer::create_torrent, peers_comunication,
        tracker_comunication::http_handler::communicate_with_tracker,
    },
    data::config_file_data::ConfigFileData,
    data::torrent_status::TorrentStatus,
    local_peer_communicator::generate_peer_id,
    logger::Logger,
};
use crate::torrent::{entry_files_management, logger};
use core::fmt;
use gtk::glib::Sender as UiSender;
use log::{debug, info, trace};
use std::error::Error;
use std::sync::mpsc::Sender as LoggerSender;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

use super::client::medatada_analyzer::MetadataError;
use super::client::tracker_comunication::http_handler::ErrorMsgHttp;
use super::data::torrent_file_data::TorrentFileData;
use super::local_peer_communicator::InteractionHandlerError;
use super::logger::LogError;
use super::pieces_handler::PiecesAssemblerError;
use super::user_interface::ui_handler::UiError;

#[derive(PartialEq, Debug)]
pub enum TorrentHandlerError {
    CreatingTorrent(MetadataError),
    UserInterface(UiError),
    CreatingLogger(LogError),
    WritingLogger(String),
    ClosingLogger(String),
    CommunicationWithTracker(ErrorMsgHttp),
    CommunicationWithPeers(InteractionHandlerError),
    PiecesHandler(PiecesAssemblerError),
    JoinHandle(String),
    SetGlobalShutDown(String),
    ReadingShutDownField(String),
}

impl fmt::Display for TorrentHandlerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for TorrentHandlerError {}

type ResultTorrent = Result<(), TorrentHandlerError>;
type JoinHandleTorrent = JoinHandle<ResultTorrent>;

fn handle_torrent(
    torrent_file: TorrentFileData,
    config_data: &ConfigFileData,
    logger_sender: &LoggerSender<String>,
    ui_sender: &UiSender<MessageUI>,
    global_shut_down: &Arc<RwLock<bool>>,
) -> ResultTorrent {
    let torrent_size = torrent_file.get_total_length();
    let torrent_status = TorrentStatus::new(torrent_size, torrent_file.total_amount_of_pieces);
    trace!("Creado estado inicial del torrent");

    let peer_id = generate_peer_id();

    info!("Iniciando comunicacion con tracker");
    let tracker_response = communicate_with_tracker(&torrent_file, config_data, peer_id.clone())
        .map_err(|err| TorrentHandlerError::CommunicationWithTracker(err))?;
    info!("Comunicacion con el tracker exitosa");

    ui_handler::update_torrent_information(
        &ui_sender,
        &torrent_file,
        &tracker_response,
        &torrent_status,
    )
    .map_err(|err| TorrentHandlerError::UserInterface(err))?;

    info!("Inicio de comunicacion con peers.");
    peers_comunication::handler::handle_general_interaction_with_peers(
        &torrent_file,
        &tracker_response,
        torrent_status,
        &config_data,
        peer_id,
        global_shut_down.clone(),
        logger_sender,
        ui_sender,
    )
    .map_err(|err| TorrentHandlerError::CommunicationWithPeers(err))?;
    Ok(())
}

fn log_torrent_error(
    torrent_name: &str,
    error: TorrentHandlerError,
    logger_sender: &LoggerSender<String>,
) -> ResultTorrent {
    info!(
        "Finalización de descarga de piezas en el torrent {:?} debido al error: {}",
        torrent_name, error
    );
    logger_sender
        .send(format!(
            "[ERROR] Finalización de descarga de piezas en el torrent {:?} debido al error: {}",
            torrent_name, error,
        ))
        .map_err(|err| TorrentHandlerError::WritingLogger(format!("{}", err)))?;
    Ok(())
}

fn log_finished_torrent(torrent_name: &str, logger_sender: &LoggerSender<String>) -> ResultTorrent {
    info!(
        "Finalización de descarga de piezas en el torrent {:?}",
        torrent_name
    );
    logger_sender
        .send(format!(
            "[END] Finalización de descarga de piezas en el torrent {:?}",
            torrent_name,
        ))
        .map_err(|err| TorrentHandlerError::WritingLogger(format!("{}", err)))?;
    Ok(())
}

fn is_shut_down_set(global_shut_down: &Arc<RwLock<bool>>) -> Result<bool, TorrentHandlerError> {
    let global_shut_down = global_shut_down
        .read()
        .map_err(|error| TorrentHandlerError::ReadingShutDownField(format!("{:?}", error)))?;
    return Ok(*global_shut_down);
}

fn set_up_logger(
    config_data: &ConfigFileData,
    torrent_file: &TorrentFileData,
) -> Result<(LoggerSender<String>, JoinHandle<()>), TorrentHandlerError> {
    let logger = Logger::new(
        config_data.get_log_path(),
        torrent_file.get_torrent_representative_name(),
    )
    .map_err(|err| TorrentHandlerError::CreatingLogger(err))?;
    Ok(logger
        .init_logger()
        .map_err(|err| TorrentHandlerError::CreatingLogger(err))?)
}

fn handle_list_of_torrents(
    files_list: Vec<String>,
    config_data: ConfigFileData,
    ui_sender: UiSender<MessageUI>,
    global_shut_down: Arc<RwLock<bool>>,
) -> JoinHandleTorrent {
    let mut current_torrent_index = 0;
    let max_torrent_index = files_list.len();

    thread::spawn(move || loop {
        if is_shut_down_set(&global_shut_down)? {
            return Ok(());
        }
        if current_torrent_index >= max_torrent_index {
            info!("No hay mas torrents para descargar, por favor cierre la pestaña.");
            return Ok(());
        }

        let file_path = files_list[current_torrent_index].clone();
        debug!("Archivo ingresado: {}", file_path);
        info!("Archivo ingresado con exito");

        let torrent_file = match create_torrent(&file_path)
            .map_err(|err| TorrentHandlerError::CreatingTorrent(err))
        {
            Ok(torrent_file) => torrent_file,
            Err(error) => {
                info!("Error al querer crear el torrent {}: {}", file_path, error);
                current_torrent_index += 1;
                continue;
            }
        };
        trace!("Almacenada y parseada información de metadata");

        ui_handler::add_torrent(&ui_sender, &torrent_file)
            .map_err(|err| TorrentHandlerError::UserInterface(err))?;

        let torrent_name = torrent_file.get_torrent_representative_name();
        let (logger_sender, logger_handler) = match set_up_logger(&config_data, &torrent_file) {
            Ok(logger) => logger,
            Err(error) => {
                info!(
                    "Error al querer crear el logger del torrent {}: {}",
                    torrent_name, error
                );
                current_torrent_index += 1;
                continue;
            }
        };

        if let Err(error) = handle_torrent(
            torrent_file,
            &config_data,
            &logger_sender,
            &ui_sender,
            &global_shut_down,
        ) {
            log_torrent_error(&torrent_name, error, &logger_sender)?;
        };
        log_finished_torrent(&torrent_name, &logger_sender)?;

        logger::close_logger(logger_handler, logger_sender)
            .map_err(|err| TorrentHandlerError::ClosingLogger(format!("{}", err)))?;

        current_torrent_index += 1;
    })
}

fn generate_file_lists(files_list: &Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut files_list_1: Vec<String> = vec![];
    let mut files_list_2: Vec<String> = vec![];

    for (i, file) in files_list.iter().enumerate() {
        if i % 2 == 0 {
            files_list_1.push(file.clone())
        } else {
            files_list_2.push(file.clone())
        }
    }
    (files_list_1, files_list_2)
}

pub fn handle_all_torrents(
    ui_sender: UiSender<MessageUI>,
    global_shut_down: &Arc<RwLock<bool>>,
) -> Result<(JoinHandleTorrent, JoinHandleTorrent), Box<dyn Error>> {
    let mut config_data = ConfigFileData::new("config.txt")?;
    info!("Archivo de configuración leido y parseado correctamente");

    let files_list = entry_files_management::create_list_files()?;

    info!("Archivo ingresado con exito");

    let (files_list_1, files_list_2) = generate_file_lists(&files_list);

    let torrent_handler_1 = handle_list_of_torrents(
        files_list_1,
        config_data.clone(),
        ui_sender.clone(),
        global_shut_down.clone(),
    );

    config_data.port += 1;
    let torrent_handler_2 = handle_list_of_torrents(
        files_list_2,
        config_data,
        ui_sender,
        global_shut_down.clone(),
    );

    Ok((torrent_handler_1, torrent_handler_2))
}
