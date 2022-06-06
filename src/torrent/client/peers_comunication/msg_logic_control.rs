//! # Modulo de logica de control para interaccion individual
//! Este modulo contiene las funciones encargadas de manejar la conexion y
//! comunicacion exclusivamente un peer indicado
//!

use crate::torrent::{
    client::client_struct::*,
    data::tracker_response_data::TrackerResponseData,
    parsers::p2p::message::{P2PMessage, PieceStatus},
};
use core::fmt;
use log::{debug, info};
use std::{error::Error, ffi::OsStr, net::TcpStream, path::Path, time::Duration};

use super::{
    handler::HandlerInteractionStatus,
    msg_receiver::{self, MsgReceiverError},
    msg_sender::{self, MsgSenderError},
};

pub const BLOCK_BYTES: u32 = 16384; //2^14 bytes

pub const SECS_READ_TIMEOUT: u64 = 120;
pub const NANOS_READ_TIMEOUT: u32 = 0;

#[derive(PartialEq, Debug, Clone)]
/// Representa un tipo de error en la comunicación general P2P con un peer individual.
pub enum MsgLogicControlError {
    ConectingWithPeer(String),
    RestartingDownload(String),
    UpdatingBitfield(String),
    LookingForPieces(String),
    CheckingAndSavingHandshake(String),
    ReceivingHanshake(String),
    ReceivingMessage(String),
    SendingHandshake(String),
    SendingMessage(String),
    UpdatingPieceStatus(String),
    StoringBlock(String),
    UpdatingFields(String),
    CalculatingServerPeerIndex(String),
    CalculatingPieceLenght(String),
}

impl fmt::Display for MsgLogicControlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for MsgLogicControlError {}

//CONNECTION
fn start_connection_with_a_peer(
    client_peer: &Client,
    server_peer_index: usize,
) -> Result<TcpStream, MsgLogicControlError> {
    if let Some(tracker_response) = &client_peer.tracker_response {
        let peer_data = match tracker_response.peers.get(server_peer_index) {
            Some(peer_data) => peer_data,
            None => {
                return Err(MsgLogicControlError::ConectingWithPeer(String::from(
                    "[MsgLogicControlError] Couldn`t find a server peer on the given index.",
                )))
            }
        };
        let stream = TcpStream::connect(&peer_data.peer_address)
            .map_err(|error| MsgLogicControlError::ConectingWithPeer(format!("{:?}", error)))?;
        stream
            .set_read_timeout(Some(Duration::new(SECS_READ_TIMEOUT, NANOS_READ_TIMEOUT)))
            .map_err(|err| MsgLogicControlError::ConectingWithPeer(format!("{:?}", err)))?;
        return Ok(stream);
    }
    Err(MsgLogicControlError::ConectingWithPeer(String::from(
        "[MsgLogicControlError] Client peer doesn`t have a tracker response.",
    )))
}

//UPDATE INFORMATION
fn update_information_according_to_the_received_msg(
    client_peer: &mut Client,
    server_peer_index: usize,
    received_msg: P2PMessage,
) -> Result<(), MsgLogicControlError> {
    match received_msg {
        P2PMessage::KeepAlive => Ok(()),
        P2PMessage::Choke => client_peer.update_peer_choking_field(server_peer_index, true),
        P2PMessage::Unchoke => client_peer.update_peer_choking_field(server_peer_index, false),
        P2PMessage::Have { piece_index } => client_peer.update_server_peer_piece_status(
            server_peer_index,
            piece_index,
            PieceStatus::ValidAndAvailablePiece,
        ),
        P2PMessage::Bitfield { bitfield } => {
            client_peer.update_peer_bitfield(bitfield, server_peer_index)
        }
        P2PMessage::Piece {
            piece_index,
            beginning_byte_index,
            block,
        } => {
            let torrent_name = &client_peer.torrent_file.name.clone();
            let path_name = Path::new(torrent_name)
                .file_stem()
                .map_or(Some("no_name"), OsStr::to_str)
                .map_or("pieces_of_no-named_torrent", |name| name);

            client_peer.store_block(piece_index, beginning_byte_index, block, path_name)?;

            debug!(
                "Nuevo estado de la pieza {}: {:?}",
                piece_index, client_peer.data_of_download.pieces_availability[0]
            );
            Ok(())
        }
        _ => Ok(()),
    }?;
    Ok(())
}

//LOOK FOR PIECES AND SEND MESSAGE
fn send_msg_according_to_peer_choking_field(
    client_peer: &mut Client,
    server_peer_index: usize,
    client_stream: &mut TcpStream,
    piece_index: u32,
) -> Result<(), MsgLogicControlError> {
    if client_peer.peer_choking(server_peer_index) {
        info!("Mensaje enviado: Interested");
        msg_sender::send_interested(client_stream).map_err(|err| {
            if let MsgSenderError::WriteToTcpStream(_) = err {
                MsgLogicControlError::ConectingWithPeer(format!("{:?}", err))
            } else {
                MsgLogicControlError::SendingMessage(format!("{:?}", err))
            }
        })?;
    } else {
        let beginning_byte_index = client_peer.calculate_beginning_byte_index(piece_index)?;
        let amount_of_bytes =
            client_peer.calculate_amount_of_bytes(piece_index, beginning_byte_index)?;
        info!("Mensaje enviado: Request[piece_index: {}, beginning_byte_index: {}. amount_of_bytes: {}]", piece_index, beginning_byte_index, amount_of_bytes);
        msg_sender::send_request(
            client_stream,
            piece_index,
            beginning_byte_index,
            amount_of_bytes,
        )
        .map_err(|err| {
            if let MsgSenderError::WriteToTcpStream(_) = err {
                MsgLogicControlError::ConectingWithPeer(format!("{:?}", err))
            } else {
                MsgLogicControlError::SendingMessage(format!("{:?}", err))
            }
        })?;
    }

    Ok(())
}

fn look_for_pieces(
    client_peer: &mut Client,
    server_peer_index: usize,
    client_stream: &mut TcpStream,
) -> Result<(), MsgLogicControlError> {
    let piece_index = match client_peer.look_for_a_missing_piece_index(server_peer_index) {
        Some(piece_index) => {
            client_peer.update_am_interested_field(server_peer_index, true)?;
            piece_index
                .try_into()
                .map_err(|error| MsgLogicControlError::LookingForPieces(format!("{:?}", error)))?
        }
        None => {
            client_peer.update_am_interested_field(server_peer_index, false)?;
            return Ok(());
        }
    };

    send_msg_according_to_peer_choking_field(
        client_peer,
        server_peer_index,
        client_stream,
        piece_index,
    )?;

    Ok(())
}

fn calculate_server_peer_index(
    client_peer: &mut Client,
    tracker_response_server_peer_index: usize,
) -> Result<usize, MsgLogicControlError> {
    let mut server_peer_id = None;
    if let Some(TrackerResponseData {
        interval: _interval,
        complete: _complete,
        incomplete: _incomplete,
        peers,
    }) = &client_peer.tracker_response
    {
        if let Some(peer_data) = peers.get(tracker_response_server_peer_index) {
            server_peer_id = peer_data.peer_id.clone();
        };
    }

    let server_peer_id = match server_peer_id {
        Some(server_peer_id) => server_peer_id,
        None => {
            return Err(MsgLogicControlError::CalculatingServerPeerIndex(
                "Couldn`t find the peer id in the given index.".to_string(),
            ))
        }
    };

    if let Some(list_of_peers_data_for_communication) =
        &client_peer.list_of_peers_data_for_communication
    {
        if let Some((server_peer_index, _peer_data)) = list_of_peers_data_for_communication
            .iter()
            .enumerate()
            .find(|&(_peer_index, peer_data)| *peer_data.peer_id == server_peer_id)
        {
            return Ok(server_peer_index);
        }
    }
    Err(MsgLogicControlError::CalculatingServerPeerIndex(
        "There is no server peer with the given peer id.".to_string(),
    ))
}

fn log_info_msg(msg: &P2PMessage) {
    match &msg {
        P2PMessage::Piece {
            piece_index,
            beginning_byte_index,
            block: _,
        } => info!(
            "Mensaje recibido: Piece[piece_index: {}, beginning_byte_index: {}]",
            piece_index, beginning_byte_index
        ),
        P2PMessage::Bitfield { bitfield: _ } => info!("Mensaje recibido: Bitfield"),
        _ => info!("Mensaje recibido: {:?}", msg),
    }
}

/// Funcion encargada de la interacción general con un peer individual.
/// Maneja toda la logica de intercambio de mensajes con dicho peer
/// para ir pidiendo y descargando por bloques al torrent correspondiente.
///
pub fn interact_with_single_peer(
    client_peer: &mut Client,
    tracker_response_server_peer_index: usize,
) -> Result<HandlerInteractionStatus, MsgLogicControlError> {
    //CONEXION CON UN PEER
    let mut client_stream =
        start_connection_with_a_peer(&*client_peer, tracker_response_server_peer_index)?;
    info!("El cliente se conecta con un peer exitosamente.");

    //ENVIO HANDSHAKE
    info!("Mensaje enviado: Handshake.");
    msg_sender::send_handshake(client_peer, &mut client_stream).map_err(|error| {
        if let MsgSenderError::WriteToTcpStream(_) = error {
            MsgLogicControlError::ConectingWithPeer(format!("{:?}", error))
        } else {
            MsgLogicControlError::SendingHandshake(format!("{:?}", error))
        }
    })?;

    //RECIBO HANDSHAKE
    let received_handshake =
        msg_receiver::receive_handshake(&mut client_stream).map_err(|error| {
            if let MsgReceiverError::ReadingFromTcpStream(_) = error {
                MsgLogicControlError::ConectingWithPeer(format!("{:?}", error))
            } else {
                MsgLogicControlError::ReceivingHanshake(format!("{:?}", error))
            }
        })?;
    info!("Mensaje recibido: Handshake.");
    client_peer
        .check_and_save_handshake_data(received_handshake, tracker_response_server_peer_index)?;

    let server_peer_index =
        calculate_server_peer_index(client_peer, tracker_response_server_peer_index)?;

    loop {
        //RECIBO UN MENSAJE
        let received_msg = msg_receiver::receive_message(&mut client_stream).map_err(|error| {
            if let MsgReceiverError::ReadingFromTcpStream(_) = error {
                MsgLogicControlError::ConectingWithPeer(format!("{:?}", error))
            } else {
                MsgLogicControlError::ReceivingMessage(format!("{:?}", error))
            }
        })?;
        log_info_msg(&received_msg);

        //ACTUALIZO MI INFORMACION SEGUN MENSAJE
        update_information_according_to_the_received_msg(
            client_peer,
            server_peer_index,
            received_msg,
        )?;

        //BUSCO SI TIENE UNA PIEZA QUE ME INTERESE Y ENVIO MENSAJE
        look_for_pieces(client_peer, server_peer_index, &mut client_stream)?;

        //VERIFICO SI DEBO CORTAR LA INTERACCION
        if !client_peer.am_interested(server_peer_index) {
            info!("Se busca un nuevo peer al cual pedirle piezas");
            return Ok(HandlerInteractionStatus::LookForAnotherPeer);
        } else if client_peer
            .data_of_download
            .pieces_availability
            .iter()
            .any(|piece| *piece == PieceStatus::ValidAndAvailablePiece)
        {
            return Ok(HandlerInteractionStatus::FinishInteraction);
        }
    }
}
