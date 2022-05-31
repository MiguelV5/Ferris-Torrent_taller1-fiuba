#![allow(dead_code)]
use crate::torrent::data::peer_data_for_communication::PeerDataForP2PCommunication;
use crate::torrent::data::tracker_response_data::PeerDataFromTrackerResponse;
use crate::torrent::parsers::p2p::constants::PSTR_STRING_HANDSHAKE;
use crate::torrent::parsers::p2p::message::PieceStatus;
use crate::torrent::{client::client_struct::*, parsers::p2p::message::P2PMessage};
use core::fmt;
use std::error::Error;
use std::net::TcpStream;
use std::vec;

use super::msg_receiver::receive_message;
use super::msg_sender::{send_interested, send_request};
use super::{msg_receiver::receive_handshake, msg_sender::send_handshake};

// otra manera de hacer un socket address:
// SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)

pub const DEFAULT_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";
pub const DEFAULT_TRACKER_ID: &str = "Tracker ID";
pub const DEFAULT_INFO_HASH: [u8; 20] = [0; 20];

/*
 * Falta:
 * - Ver los test que tienen accesos a estructura y devuelven Ok
 */

#[derive(PartialEq, Debug, Clone)]
pub enum MsgLogicControlError {
    SendingHandshake(String),
    ConectingWithPeer(String),
    CheckingAndSavingHandshake(String),
    UpdatingBitfield(String),
    LookingForPieces(String),
    ReceivingHanshake(String),
    ReceivingMessage(String),
    SendingMessage(String),
    UpdatingPieceStatus(String),
    Testing(String), //no me parece del todo bueno pero no encontre otra forma para levantar errores en test.
}

impl fmt::Display for MsgLogicControlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for MsgLogicControlError {}

//CONNECTION
fn start_connection_with_a_peers(
    peer_list: &[PeerDataFromTrackerResponse],
    server_peer_index: usize,
) -> Result<TcpStream, MsgLogicControlError> {
    let peer_data = match peer_list.get(server_peer_index) {
        Some(peer_data) => peer_data,
        None => {
            return Err(MsgLogicControlError::ConectingWithPeer(String::from(
                "[MsgLogicControlError] Couldn`t find a server peer on the given index.",
            )))
        }
    };
    let stream = TcpStream::connect(&peer_data.peer_address)
        .map_err(|error| MsgLogicControlError::ConectingWithPeer(format!("{:?}", error)))?;
    Ok(stream)
}

//HANDSHAKE
fn has_expected_peer_id(
    client_peer: &mut Client,
    server_peer_id: &str,
    server_peer_index: usize,
) -> bool {
    match client_peer.tracker_response.peers.get(server_peer_index) {
        Some(tracker_response_peer_data) => {
            if let Some(tracker_response_peer_id) = &tracker_response_peer_data.peer_id {
                tracker_response_peer_id == server_peer_id
            } else {
                true
            }
        }
        None => false,
    }
}

fn check_handshake(
    client_peer: &mut Client,
    server_protocol_str: String,
    server_info_hash: Vec<u8>,
    server_peer_id: &str,
    server_peer_index: usize,
) -> Result<(), MsgLogicControlError> {
    if (server_protocol_str != PSTR_STRING_HANDSHAKE)
        || (server_info_hash != client_peer.info_hash)
        || !has_expected_peer_id(client_peer, server_peer_id, server_peer_index)
    {
        return Err(MsgLogicControlError::CheckingAndSavingHandshake(
            "[MsgLogicControlError] The received handshake hasn`t got the expected fields."
                .to_string(),
        ));
    }
    Ok(())
}

fn save_handshake_data(client_peer: &mut Client, server_peer_id: String) {
    let new_peer = PeerDataForP2PCommunication {
        pieces_availability: None,
        peer_id: server_peer_id,
        am_interested: false,
        am_choking: true,
        peer_choking: true,
    };

    match &mut client_peer.list_of_peers_data_for_communication {
        Some(list_of_peers_data_for_communication) => {
            list_of_peers_data_for_communication.push(new_peer);
        }
        None => {
            client_peer.list_of_peers_data_for_communication = Some(vec![new_peer]);
        }
    };
}

fn check_and_save_handshake_data(
    client_peer: &mut Client,
    message: P2PMessage,
    server_peer_index: usize,
) -> Result<(), MsgLogicControlError> {
    if let P2PMessage::Handshake {
        protocol_str: server_protocol_str,
        info_hash: server_info_hash,
        peer_id: server_peer_id,
    } = message
    {
        check_handshake(
            client_peer,
            server_protocol_str,
            server_info_hash,
            &server_peer_id,
            server_peer_index,
        )?;
        save_handshake_data(client_peer, server_peer_id);
        Ok(())
    } else {
        Err(MsgLogicControlError::CheckingAndSavingHandshake(
            "[MsgLogicControlError] The received messagge is not a handshake.".to_string(),
        ))
    }
}

//BITFIELD
fn is_any_spare_bit_set(client_peer: &Client, bitfield: &[PieceStatus]) -> bool {
    return bitfield
        .iter()
        .skip(client_peer.torrent_file.total_amount_pieces)
        .any(|piece_status| *piece_status == PieceStatus::ValidAndAvailablePiece);
}

fn check_bitfield(
    client_peer: &Client,
    bitfield: &mut [PieceStatus],
) -> Result<(), MsgLogicControlError> {
    if bitfield.len() < client_peer.torrent_file.total_amount_pieces {
        return Err(MsgLogicControlError::UpdatingBitfield(
            "[MsgLogicControlError] The bitfield length is incorrect.".to_string(),
        ));
    }

    if is_any_spare_bit_set(client_peer, bitfield) {
        return Err(MsgLogicControlError::UpdatingBitfield(
            "[MsgLogicControlError] Some of the spare bits are set.".to_string(),
        ));
    }
    Ok(())
}

fn check_and_truncate_bitfield_according_to_total_amount_of_pieces(
    client_peer: &Client,
    bitfield: &mut Vec<PieceStatus>,
) -> Result<(), MsgLogicControlError> {
    check_bitfield(client_peer, bitfield)?;
    bitfield.truncate(client_peer.torrent_file.total_amount_pieces);
    Ok(())
}

fn update_peers_data_list(
    list_of_peers_data_for_communication: &mut [PeerDataForP2PCommunication],
    bitfield: Vec<PieceStatus>,
    server_peer_index: usize,
) -> Result<(), MsgLogicControlError> {
    let peer_data = list_of_peers_data_for_communication.get_mut(server_peer_index);
    match peer_data {
        Some(peer_data) => {
            peer_data.pieces_availability = Some(bitfield);
            Ok(())
        }
        None => Err(MsgLogicControlError::UpdatingBitfield(
            "[MsgLogicControlError] Couldn`t find a server peer on the given index".to_string(),
        )),
    }
}

fn update_peer_bitfield(
    client_peer: &mut Client,
    mut bitfield: Vec<PieceStatus>,
    server_peer_index: usize,
) -> Result<(), MsgLogicControlError> {
    check_and_truncate_bitfield_according_to_total_amount_of_pieces(client_peer, &mut bitfield)?;
    match &mut client_peer.list_of_peers_data_for_communication {
        Some(list_of_peers_data_for_communication) => update_peers_data_list(
            list_of_peers_data_for_communication,
            bitfield,
            server_peer_index,
        ),
        None => Err(MsgLogicControlError::UpdatingBitfield(
            "[MsgLogicControlError] Server peers list invalid access".to_string(),
        )),
    }
}

//LOOK FOR PIECES INDEX
fn server_peer_has_a_valid_and_available_piece_on_position(
    client_peer: &Client,
    server_peer_index: usize,
    position: usize,
) -> bool {
    if let Some(list_of_peers_data_for_communication) =
        &client_peer.list_of_peers_data_for_communication
    {
        if let Some(server_peer_data) = list_of_peers_data_for_communication.get(server_peer_index)
        {
            if let Some(pieces_availability) = &server_peer_data.pieces_availability {
                return pieces_availability[position] == PieceStatus::ValidAndAvailablePiece;
            }
        }
    }

    false
}

fn look_for_a_missing_piece_index(client_peer: &Client, server_peer_index: usize) -> Option<usize> {
    for (piece_index, _piece_status) in client_peer
        .data_of_download
        .pieces_availability
        .iter()
        .filter(|piece_status| **piece_status == PieceStatus::MissingPiece)
        .enumerate()
    {
        if server_peer_has_a_valid_and_available_piece_on_position(
            client_peer,
            server_peer_index,
            piece_index,
        ) {
            return Some(piece_index);
        }
    }

    None
}

fn look_for_beggining_byte_index(client_peer: &Client, piece_index: u32) -> Option<u32> {
    if let Some(PieceStatus::PartiallyDownloaded { downloaded_bytes }) = client_peer
        .data_of_download
        .pieces_availability
        .get(piece_index as usize)
    {
        return Some(*downloaded_bytes);
    }
    None
}

// UPDATING FIELDS
fn update_am_interested_field(client_peer: &mut Client, server_peer_index: usize, new_value: bool) {
    if let Some(list_of_peers_data_for_communication) =
        &mut client_peer.list_of_peers_data_for_communication
    {
        if let Some(server_peer_data) =
            list_of_peers_data_for_communication.get_mut(server_peer_index)
        {
            server_peer_data.am_interested = new_value;
        }
    }
}

fn update_peer_choking_field(client_peer: &mut Client, server_peer_index: usize, new_value: bool) {
    if let Some(list_of_peers_data_for_communication) =
        &mut client_peer.list_of_peers_data_for_communication
    {
        if let Some(server_peer_data) =
            list_of_peers_data_for_communication.get_mut(server_peer_index)
        {
            server_peer_data.peer_choking = new_value;
        }
    }
}

fn update_am_choking_field(client_peer: &mut Client, server_peer_index: usize, new_value: bool) {
    if let Some(list_of_peers_data_for_communication) =
        &mut client_peer.list_of_peers_data_for_communication
    {
        if let Some(server_peer_data) =
            list_of_peers_data_for_communication.get_mut(server_peer_index)
        {
            server_peer_data.am_choking = new_value;
        }
    }
}

// ASK FOR INFORMATION
fn peer_choking(client_peer: &Client, server_peer_index: usize) -> bool {
    if let Some(list_of_peers_data_for_communication) =
        &client_peer.list_of_peers_data_for_communication
    {
        if let Some(server_peer_data) = list_of_peers_data_for_communication.get(server_peer_index)
        {
            return server_peer_data.peer_choking;
        }
    }
    true
}

fn am_interested(client_peer: &Client, server_peer_index: usize) -> bool {
    if let Some(list_of_peers_data_for_communication) =
        &client_peer.list_of_peers_data_for_communication
    {
        if let Some(server_peer_data) = list_of_peers_data_for_communication.get(server_peer_index)
        {
            return server_peer_data.am_interested;
        }
    }
    false
}

// HAVE
fn update_server_peer_piece_status(
    client_peer: &mut Client,
    server_peer_index: usize,
    piece_index: u32,
    new_status: PieceStatus,
) -> Result<(), MsgLogicControlError> {
    if let Some(list_of_peers_data_for_communication) =
        &mut client_peer.list_of_peers_data_for_communication
    {
        if let Some(peer_data) = list_of_peers_data_for_communication.get_mut(server_peer_index) {
            if let Some(pieces_availability) = &mut peer_data.pieces_availability {
                if let Some(piece_status) =
                    pieces_availability.get_mut(usize::try_from(piece_index).map_err(|err| {
                        MsgLogicControlError::UpdatingPieceStatus(format!("{:?}", err))
                    })?)
                {
                    *piece_status = new_status;
                    return Ok(());
                }
                return Err(MsgLogicControlError::UpdatingPieceStatus(
                    "[MsgLogicControlError] Invalid piece index.".to_string(),
                ));
            }
        }
    }

    Err(MsgLogicControlError::UpdatingPieceStatus(
        "[MsgLogicControlError] Client peer invalid access.".to_string(),
    ))
}

fn react_according_to_the_received_msg(
    client_peer: &mut Client,
    server_peer_index: usize,
    received_msg: P2PMessage,
) -> Result<(), MsgLogicControlError> {
    match received_msg {
        P2PMessage::KeepAlive => {
            // deberia extender el tiempo antes de cortar la conexion
            Ok(())
        }
        P2PMessage::Choke => {
            //capaz esto podria devolver un Result<>
            update_peer_choking_field(client_peer, server_peer_index, true);
            Ok(())
        }
        P2PMessage::Unchoke => {
            //capaz esto podria devolver un Result<>
            update_peer_choking_field(client_peer, server_peer_index, false);
            Ok(())
        }
        P2PMessage::Interested => {
            //no hago nada porque estoy del lado del cliente.
            Ok(())
        }
        P2PMessage::NotInterested => {
            //no hago nada porque estoy del lado del cliente.
            Ok(())
        }
        P2PMessage::Have { piece_index } => update_server_peer_piece_status(
            client_peer,
            server_peer_index,
            piece_index,
            PieceStatus::ValidAndAvailablePiece,
        ),
        P2PMessage::Bitfield { bitfield } => {
            update_peer_bitfield(client_peer, bitfield, server_peer_index)
        }
        P2PMessage::Request {
            piece_index: _,
            beginning_byte_index: _,
            amount_of_bytes: _,
        } => {
            //no hago nada
            Ok(())
        }
        P2PMessage::Piece {
            piece_index: _,
            beginning_byte_index: _,
            block: _,
        } => {
            //save_block()
            // Miguel: [NOTA PARA DESPUES CUANDO SE NECESITE USAR THREADS] Lo que se me ocurre acá para cuando se vaya a guardar el archivo COMPLETO es que hagas una funcion en un archivo nuevo en el directorio data que sea tipo piece_collector.rs ponele; que lo que hace es ir recibiendo las piezas (completas) por un channel (o tambien se podria directamente retornar la pieza completa desde la funcion principal de este archivo en vez de  un Ok(()), y que en la funcion del handle.rs q se vayan acumulando piezas para despues llamar al collector y que vaya escribiendo el archivo). Tambien para despues, habrá que ver como adaptar la funcion principal de este archivo para que NO reciba un &mut al client, sino capaz un Arc<Mutex<Client>>
            //
            // Miguel: [NOTA PARA AHORA (ENTREGA PARCIAL)] Como solo nos piden tener una pieza, creo que lo que se busca es que se escriba esa sola piezita en un archivo para despues poder aplicarle el comando sumsha1 de linux y comparar con el valor sha1 del info_hash que viene del tracker. Entonces podrias ver de retornar la pieza en la funcion principal de ESTE archivo para asi poder pasarsela al collector para que por ahora escriba esa sola pieza como un archivito "completo" y listo.
            Ok(())
        }
        P2PMessage::Cancel {
            piece_index: _,
            beginning_byte_index: _,
            amount_of_bytes: _,
        } => {
            //no hago nada
            Ok(())
        }
        P2PMessage::Port { listen_port: _ } => {
            //no se que deberia hacer con esto pero momentaneamente nada
            Ok(())
        }
        _ => Ok(()),
    }?;
    Ok(())
}

fn look_for_pieces(
    client_peer: &mut Client,
    server_peer_index: usize,
    client_stream: &mut TcpStream,
) -> Result<(), MsgLogicControlError> {
    let piece_index = match look_for_a_missing_piece_index(client_peer, server_peer_index) {
        Some(piece_index) => {
            update_am_interested_field(client_peer, server_peer_index, true);
            piece_index
                .try_into()
                .map_err(|error| MsgLogicControlError::LookingForPieces(format!("{:?}", error)))?
        }
        None => {
            update_am_interested_field(client_peer, server_peer_index, false);
            return Ok(()); //corto la conexion pero sin error porque solamente no me interesan sus piezas
        }
    };

    //se podria modularizar eso

    //SI ESTOY CHOKE -> LE MANDO QUE ESTOY INTERESADO EN UNA PIEZA
    //SI NO ESTOY CHOKE -> LE MANDO UN REQUEST PARA UNA DE SUS PIEZAS
    if peer_choking(client_peer, server_peer_index) {
        send_interested(client_stream)
            .map_err(|err| MsgLogicControlError::SendingMessage(format!("{:?}", err)))?;
    } else {
        let beginning_byte_index =
            look_for_beggining_byte_index(&*client_peer, piece_index).unwrap_or(0);
        //cambio por clippy
        // let beginning_byte_index = match look_for_beggining_byte_index(&*client_peer, piece_index) {
        //     Some(beginning_byte_index) => beginning_byte_index,
        //     None => 0, //devolver error
        // };
        let amount_of_bytes = 4; //habria que ver de que manera calcular esto segun los bytes que faltan y bla
        send_request(
            client_stream,
            piece_index,
            beginning_byte_index,
            amount_of_bytes,
        )
        .map_err(|err| MsgLogicControlError::SendingMessage(format!("{:?}", err)))?;
    }

    Ok(())
}

//
//
//
//
// (Miguel: Creo que el "full" sobra pero lo deje por las dudas, se puede quitar si se quiere)
// FUNCION PRINCIPAL
pub fn full_interaction_with_single_peer(
    client_peer: &mut Client,
    server_peer_index: usize,
) -> Result<(), MsgLogicControlError> {
    //CONEXION CON UN PEER
    let mut client_stream =
        start_connection_with_a_peers(&client_peer.tracker_response.peers, server_peer_index)?;

    //ENVIO HANDSHAKE
    send_handshake(client_peer, &mut client_stream)
        .map_err(|error| MsgLogicControlError::SendingHandshake(format!("{:?}", error)))?;

    //RECIBO HANDSHAKE
    let received_handshake = receive_handshake(&mut client_stream)
        .map_err(|error| MsgLogicControlError::ReceivingHanshake(format!("{:?}", error)))?;
    check_and_save_handshake_data(client_peer, received_handshake, server_peer_index)?;

    loop {
        //RECIBO UN MENSAJE
        let received_msg = receive_message(&mut client_stream)
            .map_err(|error| MsgLogicControlError::ReceivingMessage(format!("{:?}", error)))?;

        //REALIZO ACCION SEGUN MENSAJE
        react_according_to_the_received_msg(client_peer, server_peer_index, received_msg)?;

        //BUSCO SI TIENE UNA PIEZA QUE ME INTERESE
        look_for_pieces(client_peer, server_peer_index, &mut client_stream)?;
        if !am_interested(client_peer, server_peer_index) {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod test_msg_logic_control {
    use super::*;
    use std::error::Error;

    use crate::torrent::data::data_of_download::{DataOfDownload, StateOfDownload};
    use crate::torrent::data::torrent_file_data::TorrentFileData;
    use crate::torrent::data::tracker_response_data::{
        PeerDataFromTrackerResponse, TrackerResponseData,
    };
    use crate::torrent::parsers::p2p::constants::PSTR_STRING_HANDSHAKE;
    use crate::torrent::parsers::p2p::message::P2PMessage;
    use std::net::SocketAddr;
    use std::str::FromStr;

    fn create_default_client_peer_with_no_server_peers() -> Result<Client, Box<dyn Error>> {
        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.to_string()),
            peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
        };

        let tracker_response = TrackerResponseData {
            interval: 0,
            complete: 1,
            incomplete: 0,
            peers: vec![server_peer],
        };
        let data_of_download = DataOfDownload {
            uploaded: 0,
            downloaded: 0,
            left: 16,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            is_single_file: true,
            name: "resulting_filename.test".to_string(),
            pieces: vec![],
            path: vec![],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 16,
            total_amount_pieces: 1,
            total_size: 16,
        };
        Ok(Client {
            peer_id: DEFAULT_CLIENT_PEER_ID.to_string(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            data_of_download,
            torrent_file,
            tracker_response,
            list_of_peers_data_for_communication: None,
        })
    }

    fn create_default_client_peer_with_a_server_peer_that_has_just_one_valid_piece(
    ) -> Result<Client, Box<dyn Error>> {
        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.to_string()),
            peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
        };
        let tracker_response = TrackerResponseData {
            interval: 0,
            complete: 1,
            incomplete: 0,
            peers: vec![server_peer],
        };
        let data_of_download = DataOfDownload {
            uploaded: 0,
            downloaded: 0,
            left: 16,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            is_single_file: true,
            name: "resulting_filename.test".to_string(),
            pieces: vec![],
            path: vec![],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 16,
            total_amount_pieces: 2,
            total_size: 32,
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
            pieces_availability: Some(vec![
                PieceStatus::MissingPiece,
                PieceStatus::ValidAndAvailablePiece,
            ]),
            am_interested: false,
            am_choking: true,
            peer_choking: true,
        };
        let list_of_peers_data_for_communication = Some(vec![server_peer_data]);
        Ok(Client {
            peer_id: DEFAULT_CLIENT_PEER_ID.to_string(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            data_of_download,
            torrent_file,
            tracker_response,
            list_of_peers_data_for_communication,
        })
    }

    fn create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces(
    ) -> Result<Client, Box<dyn Error>> {
        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.to_string()),
            peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
        };
        let tracker_response = TrackerResponseData {
            interval: 0,
            complete: 0,
            incomplete: 1,
            peers: vec![server_peer],
        };
        let data_of_download = DataOfDownload {
            uploaded: 0,
            downloaded: 0,
            left: 16,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            is_single_file: true,
            name: "resulting_filename.test".to_string(),
            pieces: vec![],
            path: vec![],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 16,
            total_amount_pieces: 2,
            total_size: 32,
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
            pieces_availability: Some(vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece]),
            am_interested: false,
            am_choking: true,
            peer_choking: true,
        };
        let list_of_peers_data_for_communication = Some(vec![server_peer_data]);
        Ok(Client {
            peer_id: DEFAULT_CLIENT_PEER_ID.to_string(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            data_of_download,
            torrent_file,
            tracker_response,
            list_of_peers_data_for_communication,
        })
    }

    mod test_check_and_save_handshake_data {
        use super::*;

        #[test]
        fn receive_a_message_that_is_not_a_handshake_error() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::KeepAlive;

            assert!(
                check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
                    .is_err()
            );

            Ok(())
        }

        #[test]
        fn receive_a_handshake_with_an_incorrect_protocol_str_error() -> Result<(), Box<dyn Error>>
        {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::Handshake {
                protocol_str: "VitTorrent protocol".to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
            };

            assert!(
                check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
                    .is_err()
            );

            Ok(())
        }

        #[test]
        fn receive_a_handshake_with_an_incorrect_info_hash_error() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: [1; 20].to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
            };

            assert!(
                check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
                    .is_err()
            );

            Ok(())
        }

        #[test]
        fn receive_a_handshake_with_an_incorrect_peer_id_error() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: "-FA0001-000000000002".to_string(),
            };

            assert!(
                check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
                    .is_err()
            );

            Ok(())
        }

        #[test]
        fn client_that_has_no_peer_ids_to_check_receive_a_valid_handshake_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;

            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            //MODIFICO EL CLIENTE PARA QUE NO TENGA LOS PEER_ID DE LOS SERVER PEER
            client_peer.tracker_response.peers = vec![PeerDataFromTrackerResponse {
                peer_id: None,
                peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
            }];

            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
            };
            let expected_peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };

            check_and_save_handshake_data(&mut client_peer, message, server_piece_index)?;

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                assert_eq!(vec![expected_peer_data], peer_data_list);
                assert_eq!(1, peer_data_list.len());
                return Ok(());
            }

            Err(Box::new(MsgLogicControlError::Testing(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn client_that_has_peer_ids_to_check_receive_a_valid_handshake_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
            };
            let expected_peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };

            check_and_save_handshake_data(&mut client_peer, message, server_peer_index)?;

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                assert_eq!(vec![expected_peer_data], peer_data_list);
                assert_eq!(1, peer_data_list.len());
                return Ok(());
            }
            Err(Box::new(MsgLogicControlError::Testing(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }
    }

    mod test_update_peer_bitfield {
        use super::*;

        #[test]
        fn update_peer_bitfield_with_less_pieces_error() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let bitfield = vec![];

            let peer_data = PeerDataForP2PCommunication {
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
                pieces_availability: None,
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };
            let peer_data_list = vec![peer_data];
            client_peer.list_of_peers_data_for_communication = Some(peer_data_list);

            assert!(update_peer_bitfield(&mut client_peer, bitfield, server_piece_index).is_err());

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                if let Some(server_peer_data) = peer_data_list.get(server_piece_index) {
                    assert!(server_peer_data.pieces_availability.is_none());
                    return Ok(());
                }
            }

            Err(Box::new(MsgLogicControlError::Testing(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn update_peer_bitfield_with_more_pieces_and_spare_bits_set_error(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let bitfield = vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece,
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::ValidAndAvailablePiece,
            ];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };
            let peer_data_list = vec![peer_data];
            client_peer.list_of_peers_data_for_communication = Some(peer_data_list);

            assert!(update_peer_bitfield(&mut client_peer, bitfield, server_piece_index).is_err());

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                if let Some(server_peer_data) = peer_data_list.get(server_piece_index) {
                    assert!(server_peer_data.pieces_availability.is_none());
                    return Ok(());
                }
            }

            Err(Box::new(MsgLogicControlError::Testing(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn update_peer_bitfield_with_the_correct_amount_of_pieces_ok() -> Result<(), Box<dyn Error>>
        {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let bitfield = vec![PieceStatus::ValidAndAvailablePiece];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };
            let peer_data_list = vec![peer_data];
            client_peer.list_of_peers_data_for_communication = Some(peer_data_list);

            update_peer_bitfield(&mut client_peer, bitfield, server_piece_index)?;

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                if let Some(server_peer_data) = peer_data_list.get(server_piece_index) {
                    if let Some(piece_availability) = &server_peer_data.pieces_availability {
                        assert_eq!(
                            vec![PieceStatus::ValidAndAvailablePiece],
                            *piece_availability
                        )
                    }
                    return Ok(());
                }
            }
            Err(Box::new(MsgLogicControlError::Testing(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn update_peer_bitfield_with_more_pieces_and_spare_bits_not_set_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let bitfield = vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
            ];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };
            let peer_data_list = vec![peer_data];
            client_peer.list_of_peers_data_for_communication = Some(peer_data_list);

            update_peer_bitfield(&mut client_peer, bitfield, server_piece_index)?;

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                if let Some(server_peer_data) = peer_data_list.get(server_piece_index) {
                    if let Some(piece_availability) = &server_peer_data.pieces_availability {
                        assert_eq!(
                            vec![PieceStatus::ValidAndAvailablePiece],
                            *piece_availability
                        );
                        return Ok(());
                    }
                }
            }
            Err(Box::new(MsgLogicControlError::Testing(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }
    }

    mod test_look_for_a_missing_piece_index {
        use super::*;

        #[test]
        fn the_server_peer_has_a_valid_and_available_piece_in_the_position_one(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let client_peer =
                create_default_client_peer_with_a_server_peer_that_has_just_one_valid_piece()?;

            assert_eq!(
                Some(1),
                look_for_a_missing_piece_index(&client_peer, server_piece_index)
            );
            Ok(())
        }

        #[test]
        fn the_server_peer_has_no_pieces() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let client_peer =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces()?;

            assert_eq!(
                None,
                look_for_a_missing_piece_index(&client_peer, server_piece_index)
            );
            Ok(())
        }
    }

    mod test_update_server_peer_piece_status {

        use super::*;

        #[test]
        fn client_peer_update_piece_status_ok() -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let server_piece_index = 1;
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces()?;

            update_server_peer_piece_status(
                &mut client_peer,
                server_peer_index,
                server_piece_index,
                PieceStatus::ValidAndAvailablePiece,
            )?;

            if let Some(list_of_peers_data_for_communication) =
                client_peer.list_of_peers_data_for_communication
            {
                if let Some(server_peer_data) =
                    list_of_peers_data_for_communication.get(server_peer_index)
                {
                    if let Some(pieces_availability) = &server_peer_data.pieces_availability {
                        assert_eq!(
                            pieces_availability.get(usize::try_from(server_piece_index)?),
                            Some(&PieceStatus::ValidAndAvailablePiece)
                        );
                        return Ok(());
                    }
                }
            }

            Err(Box::new(MsgLogicControlError::Testing(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn client_peer_cannot_update_piece_status_with_invalid_index_error(
        ) -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let server_piece_index = 2;
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces()?;

            assert_eq!(
                Err(MsgLogicControlError::UpdatingPieceStatus(
                    "[MsgLogicControlError] Invalid piece index.".to_string(),
                )),
                update_server_peer_piece_status(
                    &mut client_peer,
                    server_peer_index,
                    server_piece_index,
                    PieceStatus::ValidAndAvailablePiece,
                )
            );

            Ok(())
        }
    }
}
