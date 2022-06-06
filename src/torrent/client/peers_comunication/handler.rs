//! # Modulo de manejo de comunicación con peers
//! Este modulo contiene las funciones encargadas de controlar la logica de conexion e interaccion con todos los peers necesarios.
//!

use super::super::client_struct::Client;
use super::msg_logic_control::{interact_with_single_peer, MsgLogicControlError};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

/// Representa un tipo de estado de interaccion para saber si se debe
/// continuar o finalizar la misma
pub enum HandlerInteractionStatus {
    LookForAnotherPeer,
    FinishInteraction,
}

fn flush_data(client: &mut Client) -> Result<(), MsgLogicControlError> {
    let torrent_name = &client.torrent_file.name.clone();
    let path_name = Path::new(torrent_name)
        .file_stem()
        .map_or(Some("no_name"), OsStr::to_str)
        .map_or("pieces_of_no-named_torrent", |name| name);

    let _result = fs::remove_dir_all(format!("temp/{}", path_name))
        .map_err(|err| MsgLogicControlError::RestartingDownload(format!("{:?}", err)));
    fs::create_dir(format!("temp/{}", path_name))
        .map_err(|err| MsgLogicControlError::RestartingDownload(format!("{:?}", err)))?;

    client
        .data_of_download
        .flush_data(client.torrent_file.total_size as u64);
    Ok(())
}

fn remove_all(client: &mut Client) -> Result<(), MsgLogicControlError> {
    let torrent_name = &client.torrent_file.name.clone();
    let path_name = Path::new(torrent_name)
        .file_stem()
        .map_or(Some("no_name"), OsStr::to_str)
        .map_or("pieces_of_no-named_torrent", |name| name);

    let _result = fs::remove_dir_all(format!("temp/{}", path_name))
        .map_err(|err| MsgLogicControlError::RestartingDownload(format!("{:?}", err)));
    Ok(())
}

// FUNCION PRINCIPAL
/// Funcion encargada de manejar toda conexion y comunicación con todos los
/// peers que se hayan obtenido a partir de una respuesta de tracker e info
/// adicional del archivo .torrent correspondiente.
///
/// POR AHORA finaliza la comunicación cuando puede completar una pieza completa,
/// o en caso de error interno.
///
pub fn handle_general_interaction(client: &mut Client) -> Result<(), MsgLogicControlError> {
    // POR AHORA; LOGICA PARA COMPLETAR UNA PIEZA:
    let mut current_server_peer_index = 0;

    loop {
        if let Some(tracker_response) = &client.tracker_response {
            let max_server_peer_index = tracker_response.peers.len();
            if current_server_peer_index >= max_server_peer_index {
                return Err(MsgLogicControlError::ConectingWithPeer(
                    "No se pudo conectar con ningun peer para completar la pieza".to_string(),
                ));
            }
        };

        match interact_with_single_peer(client, current_server_peer_index) {
            Ok(HandlerInteractionStatus::LookForAnotherPeer) => {
                current_server_peer_index += 1;
                flush_data(client)?;
                continue;
            }
            Ok(HandlerInteractionStatus::FinishInteraction) => {
                return Ok(());
            }
            Err(MsgLogicControlError::ConectingWithPeer(_)) => {
                current_server_peer_index += 1;
                flush_data(client)?;
                continue;
            }
            Err(another_err) => {
                remove_all(client)?;
                return Err(another_err);
            }
        };
    }
}
