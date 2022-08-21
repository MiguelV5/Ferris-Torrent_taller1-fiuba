//! # Modulo de manejo de comunicación con peers
//! Este modulo contiene las funciones encargadas de controlar la logica de conexion e interaccion con todos los peers necesarios.
//!

use log::{debug, info};

use crate::torrent::client::pieces_assembling_handler;
use crate::torrent::data::config_file_data::ConfigFileData;
use crate::torrent::data::{
    torrent_status::TorrentStatus, tracker_response_data::TrackerResponseData,
};
use crate::torrent::user_interface::constants::MessageUI;
use crate::torrent::user_interface::ui_sender_handler;
use gtk::glib::Sender as UiSender;
use shared::torrent_file_data::TorrentFileData;
use std::net::TcpListener;
use std::sync::mpsc::Sender as LoggerSender;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;
use std::{fs, thread};

use super::local_peer_communicator::{
    InteractionHandlerError, InteractionHandlerErrorKind, InteractionHandlerStatus,
    LocalPeerCommunicator,
};

type ResultInteraction<T> = Result<T, InteractionHandlerError>;
type JoinHandleInteraction<T> = JoinHandle<ResultInteraction<T>>;

type PeerId = Vec<u8>;
type ExternalPeerAddres = String;

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

fn is_shut_down_set(shut_down: &Arc<RwLock<bool>>) -> Result<bool, InteractionHandlerError> {
    let global_shut_down = shut_down
        .read()
        .map_err(|error| InteractionHandlerError::ReadingShutDownField(format!("{:?}", error)))?;
    Ok(*global_shut_down)
}

fn set_shut_down(shut_down: Arc<RwLock<bool>>) -> Result<(), InteractionHandlerError> {
    let mut shut_down = shut_down
        .write()
        .map_err(|error| InteractionHandlerError::WritingShutDownField(format!("{:?}", error)))?;
    *shut_down = true;
    Ok(())
}

///
/// Funcion encargada de realizar la interaccion con unico peer dentro de un thread. Esta interaccion se
/// comienza con el protocolo correspondiente a un server.
/// La funcion finaliza cuando se activa el shutdown global, local, cuando se obtienen todas las piezas posibles
/// a partir del peer con el cual no estamos comunicando o en caso de error.
/// Es importante remarcar que no todos los tipos de errores detienen la comunicacion con los peers, dado que
/// existen errores recuperables (los cuales no afectan la continuacion de la descarga de piezas a traves de otros
/// medios) y los errores irrecuperables (los cuales detienen completamente la descarga del torrent e imprimen un fallo tanto por consola como en el archivo logs)
///
fn handle_interaction_starting_as_server(
    read_only_data: (TorrentFileData, ConfigFileData, PeerId, ExternalPeerAddres),
    torrent_status: Arc<RwLock<TorrentStatus>>,
    logger_sender: LoggerSender<String>,
    ui_sender: UiSender<MessageUI>,
    global_shut_down: Arc<RwLock<bool>>,
    local_shut_down: Arc<RwLock<bool>>,
) -> JoinHandleInteraction<()> {
    let (torrent_file_data, config_data, peer_id, address) = read_only_data;
    thread::spawn(move || {
        let listener = TcpListener::bind(address)
            .map_err(|error| InteractionHandlerError::ConectingWithPeer(format!("{}", error)))?;
        listener
            .set_nonblocking(true)
            .map_err(|error| InteractionHandlerError::ConectingWithPeer(format!("{}", error)))?;

        loop {
            if let Ok((stream, external_peer_addr)) = listener.accept() {
                let mut local_peer = match LocalPeerCommunicator::start_communication_as_server(
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
                        set_shut_down(local_shut_down)?;
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
                        pieces_assembling_handler::assemble_all_completed_pieces(
                            config_data.get_download_path(),
                            &torrent_file_data,
                        )
                        .map_err(|err| {
                            InteractionHandlerError::PiecesHandler(format!("{}", err))
                        })?;
                        set_shut_down(local_shut_down)?;
                        Ok(())
                    }
                    Ok(InteractionHandlerStatus::LookForAnotherPeer) => {
                        ui_sender_handler::remove_external_peer(
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
                        ui_sender_handler::remove_external_peer(
                            &ui_sender,
                            &torrent_file_data,
                            &local_peer.external_peer_data,
                        )
                        .map_err(|error| InteractionHandlerError::UiError(format!("{}", error)))?;
                        continue;
                    }
                    Err(InteractionHandlerErrorKind::Unrecoverable(err)) => {
                        remove_all(&torrent_file_data)?;
                        set_shut_down(local_shut_down)?;
                        Err(err)
                    }
                };

                ui_sender_handler::remove_external_peer(
                    &ui_sender,
                    &torrent_file_data,
                    &local_peer.external_peer_data,
                )
                .map_err(|error| InteractionHandlerError::UiError(format!("{}", error)))?;
                return interaction_result;
            } else {
                if is_shut_down_set(&global_shut_down)? || is_shut_down_set(&local_shut_down)? {
                    return Ok(());
                }
                thread::sleep(Duration::from_secs(1));
                continue;
            }
        }
    })
}

fn generate_list_of_connected_peers(
    tracker_response: &TrackerResponseData,
) -> (Vec<usize>, Vec<usize>) {
    let mut list_connected_peers_1: Vec<usize> = vec![];
    let mut list_connected_peers_2: Vec<usize> = vec![];
    for i in 0..tracker_response.get_total_amount_peers() {
        if i % 2 == 0 {
            list_connected_peers_1.push(i)
        } else {
            list_connected_peers_2.push(i)
        }
    }
    (list_connected_peers_1, list_connected_peers_2)
}

///
/// Funcion encargada de realizar la interaccion con unico peer dentro de un thread. Esta interaccion se
/// comienza con el protocolo correspondiente a un cliente.
/// La funcion finaliza cuando se activa el shutdown global, local, cuando se obtienen todas las piezas posibles
/// a partir del peer con el cual no estamos comunicando o en caso de error.
/// Es importante remarcar que no todos los tipos de errores detienen la comunicacion con los peers, dado que
/// existen errores recuperables (los cuales no afectan la continuacion de la descarga de piezas a traves de otros
/// medios) y los errores irrecuperables (los cuales detienen completamente la descarga del torrent e imprimen un fallo tanto por consola como en el archivo logs)
///
fn handle_interaction_starting_as_client(
    read_only_data: (TorrentFileData, TrackerResponseData, ConfigFileData, PeerId),
    torrent_status: Arc<RwLock<TorrentStatus>>,
    mut list_connected_peers: Vec<usize>,
    logger_sender: LoggerSender<String>,
    ui_sender: UiSender<MessageUI>,
    global_shut_down: Arc<RwLock<bool>>,
    local_shut_down: Arc<RwLock<bool>>,
) -> JoinHandleInteraction<()> {
    let (torrent_file_data, tracker_response, config_data, peer_id) = read_only_data;
    thread::spawn(move || loop {
        if list_connected_peers.is_empty() {
            set_shut_down(local_shut_down)?;
            return Err(InteractionHandlerError::ConectingWithPeer(
                "No peers left to connect.".to_string(),
            ));
        }
        if is_shut_down_set(&global_shut_down)? {
            info!(
                "Shut down seguro del torrent {}.",
                torrent_file_data.get_torrent_representative_name()
            );
            return Ok(());
        }

        let current_peer_index = list_connected_peers[0];
        let mut local_peer = match LocalPeerCommunicator::start_communication_as_client(
            &torrent_file_data,
            &tracker_response,
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
                pieces_assembling_handler::assemble_all_completed_pieces(
                    config_data.get_download_path(),
                    &torrent_file_data,
                )
                .map_err(|err| InteractionHandlerError::PiecesHandler(format!("{}", err)))?;
                let mut local_shut_down = local_shut_down
                    .write()
                    .map_err(|err| InteractionHandlerError::UiError(format!("{}", err)))?;
                *local_shut_down = true;
                Ok(())
            }
            Ok(InteractionHandlerStatus::LookForAnotherPeer) => {
                let index = list_connected_peers.remove(0);
                list_connected_peers.push(index);
                let mut torrent_status = torrent_status.write().map_err(|error| {
                    InteractionHandlerError::UpdatingWasRequestedField(format!("{:?}", error))
                })?;
                torrent_status.set_all_pieces_as_not_requested();
                ui_sender_handler::remove_external_peer(
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
                ui_sender_handler::remove_external_peer(
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

        ui_sender_handler::remove_external_peer(
            &ui_sender,
            &torrent_file_data,
            &local_peer.external_peer_data,
        )
        .map_err(|error| InteractionHandlerError::UiError(format!("{}", error)))?;

        return interaction_result;
    })
}

///
/// FUNCION PRINCIPAL
/// Funcion encargada de manejar toda conexion y comunicación con todos los
/// peers que se hayan obtenido a partir de una respuesta de tracker e info
/// adicional del archivo .torrent correspondiente.
/// (***Comportandose como Cliente y como Server por la caracteristica hibrida que poseen los LocalPeerCommunicator***)
///
/// Finaliza la conexion en caso de activarse el shutdown global, en caso de completar todo el archivo
/// o en caso de error interno.
///
pub fn handle_general_interaction_with_peers(
    read_only_data: (
        &TorrentFileData,
        &TrackerResponseData,
        &ConfigFileData,
        PeerId,
    ),
    torrent_status: TorrentStatus,
    global_shut_down: Arc<RwLock<bool>>,
    logger_sender: &LoggerSender<String>,
    ui_sender: &UiSender<MessageUI>,
) -> Result<(), InteractionHandlerError> {
    let (torrent_file_data, tracker_response, config_data, peer_id) = read_only_data;
    set_up_directory(torrent_file_data)?;
    let torrent_status = Arc::new(RwLock::new(torrent_status));
    let address = generate_address(config_data);
    let (list_connected_peers_1, list_connected_peers_2) =
        generate_list_of_connected_peers(tracker_response);

    let local_shut_down = Arc::new(RwLock::new(false));

    let handler_local_peer_0 = handle_interaction_starting_as_server(
        (
            torrent_file_data.clone(),
            config_data.clone(),
            peer_id.clone(),
            address,
        ),
        torrent_status.clone(),
        logger_sender.clone(),
        ui_sender.clone(),
        global_shut_down.clone(),
        local_shut_down.clone(),
    );

    let handler_local_peer_1 = handle_interaction_starting_as_client(
        (
            torrent_file_data.clone(),
            tracker_response.clone(),
            config_data.clone(),
            peer_id.clone(),
        ),
        torrent_status.clone(),
        list_connected_peers_1,
        logger_sender.clone(),
        ui_sender.clone(),
        global_shut_down.clone(),
        local_shut_down.clone(),
    );

    let handler_local_peer_2 = handle_interaction_starting_as_client(
        (
            torrent_file_data.clone(),
            tracker_response.clone(),
            config_data.clone(),
            peer_id,
        ),
        torrent_status,
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
