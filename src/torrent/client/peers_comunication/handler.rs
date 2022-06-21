//! # Modulo de manejo de comunicación con peers
//! Este modulo contiene las funciones encargadas de controlar la logica de conexion e interaccion con todos los peers necesarios.
//!

use log::{debug, info, trace};

use crate::torrent::data::{
    torrent_file_data::TorrentFileData, torrent_status::TorrentStatus,
    tracker_response_data::TrackerResponseData,
};
use crate::torrent::local_peer::{
    InteractionHandlerError, InteractionHandlerErrorKind, InteractionHandlerStatus, LocalPeer,
};

use std::fs;

pub const BLOCK_BYTES: u32 = 16384; //2^14 bytes

pub const SECS_READ_TIMEOUT: u64 = 120;
pub const NANOS_READ_TIMEOUT: u32 = 0;

fn set_up_directory(torrent_file_data: &TorrentFileData) -> Result<(), InteractionHandlerError> {
    info!("Creo un directorio para guardar piezas");
    let torrent_path = torrent_file_data.get_torrent_representative_name();
    let _result = fs::remove_dir_all(format!("temp/{}", torrent_path));
    fs::create_dir(format!("temp/{}", torrent_path))
        .map_err(|err| InteractionHandlerError::SetUpDirectory(format!("{}", err)))?;
    Ok(())
}

fn flush_data(
    torrent_file_data: &TorrentFileData,
    torrent_status: &mut TorrentStatus,
) -> Result<(), InteractionHandlerError> {
    torrent_status.flush_data(torrent_file_data.total_length as u64);
    Ok(())
}

fn remove_all(torrent_file_data: &TorrentFileData) -> Result<(), InteractionHandlerError> {
    let torrent_path = torrent_file_data.get_torrent_representative_name();
    let _result = fs::remove_dir_all(format!("temp/{}", torrent_path))
        .map_err(|err| InteractionHandlerError::RestartingDownload(format!("{}", err)));

    Ok(())
}

// FUNCION PRINCIPAL
/// Funcion encargada de manejar toda conexion y comunicación con todos los
/// peers que se hayan obtenido a partir de una respuesta de tracker e info
/// adicional del archivo .torrent correspondiente.
/// (***Comportandose como LocalPeer de rol: Client***)
///
/// POR AHORA finaliza la comunicación cuando puede completar una pieza completa,
/// o en caso de error interno.
///
pub fn handle_general_interaction_as_client(
    torrent_file_data: &TorrentFileData,
    tracker_response_data: &TrackerResponseData,
    torrent_status: &mut TorrentStatus,
    peer_id: Vec<u8>,
) -> Result<(), InteractionHandlerError> {
    // POR AHORA; LOGICA PARA COMPLETAR UNA PIEZA:
    let mut current_peer_index = 0;

    set_up_directory(torrent_file_data)?;

    loop {
        let max_server_peer_index = tracker_response_data.peers.len();
        if current_peer_index >= max_server_peer_index {
            return Err(InteractionHandlerError::ConectingWithPeer(
                "No se pudo conectar con peers suficientes para completar el torrent".to_string(),
            ));
        };

        let mut local_peer = match LocalPeer::start_communication(
            torrent_file_data,
            tracker_response_data,
            current_peer_index,
            peer_id.clone(),
        ) {
            Ok(local_peer) => local_peer,
            Err(InteractionHandlerErrorKind::Recoverable(err)) => {
                trace!("{:?}", err);
                current_peer_index += 1;
                continue;
            }
            Err(InteractionHandlerErrorKind::Unrecoverable(err)) => {
                remove_all(torrent_file_data)?;
                return Err(err);
            }
        };
        match local_peer.interact_with_peer(torrent_file_data, torrent_status) {
            Ok(InteractionHandlerStatus::LookForAnotherPeer) => {
                current_peer_index += 1;
                flush_data(torrent_file_data, torrent_status)?;
                continue;
            }
            Ok(InteractionHandlerStatus::FinishInteraction) => {
                return Ok(());
            }
            Err(InteractionHandlerErrorKind::Recoverable(err)) => {
                debug!(
                    "Ocurrió un error recuperable en la comunicacion con peers: {:?}",
                    err
                );
                current_peer_index += 1;
                flush_data(torrent_file_data, torrent_status)?;
                continue;
            }
            Err(InteractionHandlerErrorKind::Unrecoverable(err)) => {
                remove_all(torrent_file_data)?;
                return Err(err);
            }
        };
    }
}
