//! # Modulo de manejo de comunicación con peers
//! Este modulo contiene las funciones encargadas de controlar la logica de conexion e interaccion con todos los peers necesarios.
//!

use log::{debug, info};

use crate::torrent::data::config_file_data::ConfigFileData;
use crate::torrent::data::{
    torrent_file_data::TorrentFileData, torrent_status::TorrentStatus,
    tracker_response_data::TrackerResponseData,
};
use crate::torrent::local_peer_communicator::{
    InteractionHandlerError, InteractionHandlerErrorKind, InteractionHandlerStatus,
    LocalPeerCommunicator,
};
use crate::torrent::pieces_handler;
use crate::torrent::user_interface::constants::MessageUI;
use crate::torrent::user_interface::ui_handler;
use gtk::glib::Sender as UiSender;
use std::net::TcpListener;
use std::sync::mpsc::Sender as LoggerSender;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;
use std::{fs, thread};

type ResultInteraction<T> = Result<T, InteractionHandlerError>;
type JoinHandleInteraction<T> = JoinHandle<ResultInteraction<T>>;

pub const BLOCK_BYTES: u32 = 16384; //2^14 bytes

pub const PUBLIC_IP: &str = "0.0.0.0:";

fn generate_address(config_data: &ConfigFileData) -> String {
    let address = PUBLIC_IP.to_string();
    let port = config_data.get_port().to_string();
    address + &port
}

fn set_up_directory(torrent_file_data: &TorrentFileData) -> ResultInteraction<()> {
    info!("Creo un directorio para guardar piezas");
    let torrent_path = torrent_file_data.get_torrent_representative_name();
    let _unused_result = fs::remove_dir_all(format!("temp/{}", torrent_path));
    fs::create_dir(format!("temp/{}", torrent_path))
        .map_err(|err| InteractionHandlerError::SetUpDirectory(format!("{}", err)))?;
    Ok(())
}

fn remove_all(torrent_file_data: &TorrentFileData) -> ResultInteraction<()> {
    let torrent_path = torrent_file_data.get_torrent_representative_name();
    let _unused_result = fs::remove_dir_all(format!("temp/{}", torrent_path)).map_err(|err| {
        InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::RestartingDownload(
            format!("{}", err),
        ))
    });

    Ok(())
}

fn is_global_shut_down_set(
    global_shut_down: &Arc<RwLock<bool>>,
) -> Result<bool, InteractionHandlerError> {
    let global_shut_down = global_shut_down
        .read()
        .map_err(|error| InteractionHandlerError::ReadingShutDownField(format!("{:?}", error)))?;
    return Ok(*global_shut_down);
}

fn handle_interaction_with_new_peers(
    torrent_file_data: TorrentFileData,
    torrent_status: Arc<RwLock<TorrentStatus>>,
    config_data: ConfigFileData,
    peer_id: Vec<u8>,
    address: String,
    logger_sender: LoggerSender<String>,
    ui_sender: UiSender<MessageUI>,
    global_shut_down: Arc<RwLock<bool>>,
    local_shut_down: Arc<RwLock<bool>>,
) -> JoinHandleInteraction<()> {
    thread::spawn(move || {
        let listener = TcpListener::bind(address)
            .map_err(|error| InteractionHandlerError::ConectingWithPeer(format!("{}", error)))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| InteractionHandlerError::ConectingWithPeer(format!("{}", error)))?;

        loop {
            if let Ok((stream, external_peer_addr)) = listener.accept() {
                let mut local_peer = match LocalPeerCommunicator::start_communication_with_new_peer(
                    &torrent_file_data,
                    peer_id.clone(),
                    stream,
                    external_peer_addr,
                    logger_sender.clone(),
                    ui_sender.clone(),
                ) {
                    Ok(local_peer) => local_peer,
                    Err(InteractionHandlerErrorKind::Recoverable(err)) => {
                        debug!("Recoverable error in the peers communication: {:?}", err);
                        continue;
                    }
                    Err(InteractionHandlerErrorKind::Unrecoverable(err)) => {
                        remove_all(&torrent_file_data)?;
                        let mut local_shut_down = local_shut_down.write().map_err(|error| {
                            InteractionHandlerError::ReadingShutDownField(format!("{:?}", error))
                        })?;
                        *local_shut_down = true;
                        return Err(err);
                    }
                };
                let interaction_result = match local_peer.interact_with_peer(
                    &torrent_file_data,
                    &torrent_status,
                    &global_shut_down,
                    &local_shut_down,
                ) {
                    Ok(InteractionHandlerStatus::SecureLocalShutDown) => {
                        remove_all(&torrent_file_data)?;
                        Ok(())
                    }
                    Ok(InteractionHandlerStatus::SecureGlobalShutDown) => Ok(()),
                    Ok(InteractionHandlerStatus::FinishInteraction) => {
                        pieces_handler::assemble_all_completed_pieces(
                            config_data.get_download_path(),
                            &torrent_file_data,
                        )
                        .map_err(|err| {
                            InteractionHandlerError::PiecesHandler(format!("{}", err))
                        })?;
                        return Ok(());
                    }
                    Ok(InteractionHandlerStatus::LookForAnotherPeer) => {
                        let mut torrent_status = torrent_status.write().map_err(|error| {
                            InteractionHandlerError::UpdatingWasRequestedField(format!(
                                "{:?}",
                                error
                            ))
                        })?;
                        torrent_status.set_all_pieces_as_not_requested();
                        ui_handler::remove_external_peer(
                            &ui_sender,
                            &torrent_file_data,
                            &local_peer.external_peer_data,
                        )
                        .map_err(|error| InteractionHandlerError::UiError(format!("{}", error)))?;
                        continue;
                    }
                    Err(InteractionHandlerErrorKind::Recoverable(err)) => {
                        debug!("Recoverable error in the peers communication: {:?}", err);
                        let mut torrent_status = torrent_status.write().map_err(|error| {
                            InteractionHandlerError::UpdatingWasRequestedField(format!(
                                "{:?}",
                                error
                            ))
                        })?;
                        torrent_status.set_all_pieces_as_not_requested();
                        ui_handler::remove_external_peer(
                            &ui_sender,
                            &torrent_file_data,
                            &local_peer.external_peer_data,
                        )
                        .map_err(|error| InteractionHandlerError::UiError(format!("{}", error)))?;
                        continue;
                    }
                    Err(InteractionHandlerErrorKind::Unrecoverable(err)) => {
                        remove_all(&torrent_file_data)?;
                        let mut local_shut_down = local_shut_down.write().map_err(|error| {
                            InteractionHandlerError::ReadingShutDownField(format!("{:?}", error))
                        })?;
                        *local_shut_down = true;
                        Err(err)
                    }
                };

                ui_handler::remove_external_peer(
                    &ui_sender,
                    &torrent_file_data,
                    &local_peer.external_peer_data,
                )
                .map_err(|error| InteractionHandlerError::UiError(format!("{}", error)))?;
                return interaction_result;
            } else {
                if is_global_shut_down_set(&global_shut_down)? {
                    return Ok(());
                }
                thread::sleep(Duration::from_secs(1));
                continue;
            }
        }
    })
}

fn generate_list_of_connected_peers(
    tracker_response_data: &TrackerResponseData,
) -> (Vec<usize>, Vec<usize>) {
    let mut list_connected_peers_1: Vec<usize> = vec![];
    let mut list_connected_peers_2: Vec<usize> = vec![];
    for i in 0..tracker_response_data.get_total_amount_peers() {
        if i % 2 == 0 {
            list_connected_peers_1.push(i)
        } else {
            list_connected_peers_2.push(i)
        }
    }
    (list_connected_peers_1, list_connected_peers_2)
}

fn handle_interaction_with_torrent_peers(
    torrent_file_data: TorrentFileData,
    tracker_response_data: TrackerResponseData,
    torrent_status: Arc<RwLock<TorrentStatus>>,
    config_data: ConfigFileData,
    peer_id: Vec<u8>,
    mut list_connected_peers: Vec<usize>,
    logger_sender: LoggerSender<String>,
    ui_sender: UiSender<MessageUI>,
    global_shut_down: Arc<RwLock<bool>>,
    local_shut_down: Arc<RwLock<bool>>,
) -> JoinHandleInteraction<()> {
    thread::spawn(move || loop {
        if list_connected_peers.is_empty() {
            return Err(InteractionHandlerError::ConectingWithPeer(
                "No peers left to connect.".to_string(),
            ));
        }
        if is_global_shut_down_set(&global_shut_down)? {
            info!(
                "Shut down seguro del torrent {}.",
                torrent_file_data.get_torrent_representative_name()
            );
            return Ok(());
        }

        let current_peer_index = list_connected_peers[0];
        let mut local_peer = match LocalPeerCommunicator::start_communication_with_a_torrent_peer(
            &torrent_file_data,
            &tracker_response_data,
            current_peer_index,
            peer_id.clone(),
            logger_sender.clone(),
            ui_sender.clone(),
        ) {
            Ok(local_peer) => local_peer,
            Err(InteractionHandlerErrorKind::Recoverable(err)) => {
                list_connected_peers.remove(0);
                debug!("Recoverable error in the peers communication: {:?}", err);
                continue;
            }
            Err(InteractionHandlerErrorKind::Unrecoverable(err)) => {
                remove_all(&torrent_file_data)?;
                let mut local_shut_down = local_shut_down.write().map_err(|error| {
                    InteractionHandlerError::ReadingShutDownField(format!("{:?}", error))
                })?;
                *local_shut_down = true;
                return Err(err);
            }
        };

        let interaction_result = match local_peer.interact_with_peer(
            &torrent_file_data,
            &torrent_status,
            &global_shut_down,
            &local_shut_down,
        ) {
            Ok(InteractionHandlerStatus::SecureLocalShutDown) => {
                remove_all(&torrent_file_data)?;
                Ok(())
            }
            Ok(InteractionHandlerStatus::SecureGlobalShutDown) => Ok(()),
            Ok(InteractionHandlerStatus::FinishInteraction) => {
                pieces_handler::assemble_all_completed_pieces(
                    config_data.get_download_path(),
                    &torrent_file_data,
                )
                .map_err(|err| InteractionHandlerError::PiecesHandler(format!("{}", err)))?;
                return Ok(());
            }
            Ok(InteractionHandlerStatus::LookForAnotherPeer) => {
                let index = list_connected_peers.remove(0);
                list_connected_peers.push(index);
                let mut torrent_status = torrent_status.write().map_err(|error| {
                    InteractionHandlerError::UpdatingWasRequestedField(format!("{:?}", error))
                })?;
                torrent_status.set_all_pieces_as_not_requested();
                ui_handler::remove_external_peer(
                    &ui_sender,
                    &torrent_file_data,
                    &local_peer.external_peer_data,
                )
                .map_err(|error| InteractionHandlerError::UiError(format!("{}", error)))?;
                continue;
            }
            Err(InteractionHandlerErrorKind::Recoverable(err)) => {
                list_connected_peers.remove(0);
                debug!("Recoverable error in the peers communication: {:?}", err);
                let mut torrent_status = torrent_status.write().map_err(|error| {
                    InteractionHandlerError::UpdatingWasRequestedField(format!("{:?}", error))
                })?;
                torrent_status.set_all_pieces_as_not_requested();
                ui_handler::remove_external_peer(
                    &ui_sender,
                    &torrent_file_data,
                    &local_peer.external_peer_data,
                )
                .map_err(|error| InteractionHandlerError::UiError(format!("{}", error)))?;
                continue;
            }
            Err(InteractionHandlerErrorKind::Unrecoverable(err)) => {
                remove_all(&torrent_file_data)?;
                let mut local_shut_down = local_shut_down.write().map_err(|error| {
                    InteractionHandlerError::ReadingShutDownField(format!("{:?}", error))
                })?;
                *local_shut_down = true;
                Err(err)
            }
        };

        ui_handler::remove_external_peer(
            &ui_sender,
            &torrent_file_data,
            &local_peer.external_peer_data,
        )
        .map_err(|error| InteractionHandlerError::UiError(format!("{}", error)))?;

        return interaction_result;
    })
}

// FUNCION PRINCIPAL
/// Funcion encargada de manejar toda conexion y comunicación con todos los
/// peers que se hayan obtenido a partir de una respuesta de tracker e info
/// adicional del archivo .torrent correspondiente.
/// (***Comportandose como LocalPeerCommunicator de rol: Client***)
///
/// POR AHORA finaliza la comunicación cuando puede completar una pieza completa,
/// o en caso de error interno.
///
pub fn handle_general_interaction_with_peers(
    torrent_file_data: &TorrentFileData,
    tracker_response_data: &TrackerResponseData,
    torrent_status: TorrentStatus,
    config_data: &ConfigFileData,
    peer_id: Vec<u8>,
    global_shut_down: Arc<RwLock<bool>>,
    logger_sender: &LoggerSender<String>,
    ui_sender: &UiSender<MessageUI>,
) -> Result<(), InteractionHandlerError> {
    set_up_directory(&torrent_file_data)?;
    let torrent_status = Arc::new(RwLock::new(torrent_status));
    let address = generate_address(config_data);
    let (list_connected_peers_1, list_connected_peers_2) =
        generate_list_of_connected_peers(&tracker_response_data);

    let local_shut_down = Arc::new(RwLock::new(false));

    let handler_local_peer_0 = handle_interaction_with_new_peers(
        torrent_file_data.clone(),
        torrent_status.clone(),
        config_data.clone(),
        peer_id.clone(),
        address,
        logger_sender.clone(),
        ui_sender.clone(),
        global_shut_down.clone(),
        local_shut_down.clone(),
    );

    let handler_local_peer_1 = handle_interaction_with_torrent_peers(
        torrent_file_data.clone(),
        tracker_response_data.clone(),
        torrent_status.clone(),
        config_data.clone(),
        peer_id.clone(),
        list_connected_peers_1,
        logger_sender.clone(),
        ui_sender.clone(),
        global_shut_down.clone(),
        local_shut_down.clone(),
    );

    let handler_local_peer_2 = handle_interaction_with_torrent_peers(
        torrent_file_data.clone(),
        tracker_response_data.clone(),
        torrent_status,
        config_data.clone(),
        peer_id,
        list_connected_peers_2,
        logger_sender.clone(),
        ui_sender.clone(),
        global_shut_down,
        local_shut_down,
    );

    let result_local_peer_1 = handler_local_peer_0.join().map_err(|_| {
        InteractionHandlerError::JoinHandle(
            "[InteractionHandlerError] Join handle error".to_string(),
        )
    });
    let result_local_peer_2 = handler_local_peer_1.join().map_err(|_| {
        InteractionHandlerError::JoinHandle(
            "[InteractionHandlerError] Join handle error".to_string(),
        )
    });
    let result_local_peer_3 = handler_local_peer_2.join().map_err(|_| {
        InteractionHandlerError::JoinHandle(
            "[InteractionHandlerError] Join handle error".to_string(),
        )
    });

    result_local_peer_1??;
    result_local_peer_2??;
    result_local_peer_3??;

    Ok(())
}
