#![allow(dead_code)]
use crate::torrent::data::peers_data::{PeerData, PeersDataList};
use crate::torrent::data::tracker_response_data::TrackerResponsePeerData;
use crate::torrent::parsers::p2p::constants::PSTR_STRING_HANDSHAKE;
use crate::torrent::parsers::p2p::message::PieceStatus;
use crate::torrent::{client::client_struct::*, parsers::p2p::message::P2PMessage};
use std::net::TcpStream;
use std::vec;

use super::msg_receiver::receive_message;
use super::msg_sender::send_interested;
use super::{msg_receiver::receive_handshake, msg_sender::send_handshake};

// otra manera de hacer un socket address:
// SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)

pub const DEFAULT_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";
pub const DEFAULT_TRACKER_ID: &str = "Tracker ID";
pub const DEFAULT_INFO_HASH: [u8; 20] = [0; 20];

//CONNECTION
fn start_connection_with_a_peers(
    peer_list: &[TrackerResponsePeerData],
    server_peer_index: usize,
) -> Result<TcpStream, ClientError> {
    let peer_data = match peer_list.get(server_peer_index) {
        Some(peer_data) => peer_data,
        None => {
            return Err(ClientError::ConectingWithPeer(String::from(
                "No se encontró ningun peer en el indice dado",
            )))
        }
    };
    let stream = TcpStream::connect(&peer_data.peer_address)
        .map_err(|error| ClientError::ConectingWithPeer(format!("{:?}", error)))?;
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
) -> Result<(), ClientError> {
    if (server_protocol_str != PSTR_STRING_HANDSHAKE)
        || (server_info_hash != client_peer.client_data.info_hash)
        || !has_expected_peer_id(client_peer, server_peer_id, server_peer_index)
    {
        return Err(ClientError::CheckingAndSavingHandshake(
            "El handshake recibido no posee los campos esperados.".to_string(),
        ));
    }
    Ok(())
}

fn save_handshake_data(client_peer: &mut Client, server_peer_id: String) {
    let new_peer = PeerData {
        pieces_availability: None,
        peer_id: server_peer_id,
        am_interested: false,
        am_chocking: true,
        peer_choking: true,
    };

    match &mut client_peer.peers_data_list {
        Some(peers_data_list) => {
            peers_data_list.data_list.push(new_peer);
            peers_data_list.total_amount_of_peers += 1;
        }
        None => {
            client_peer.peers_data_list = Some(PeersDataList {
                data_list: vec![new_peer],
                total_amount_of_peers: 1,
            });
        }
    };
}

fn check_and_save_handshake_data(
    client_peer: &mut Client,
    message: P2PMessage,
    server_peer_index: usize,
) -> Result<(), ClientError> {
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
        Err(ClientError::CheckingAndSavingHandshake(
            "El mensaje p2p recibido no es un handshake".to_string(),
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

fn check_bitfield(client_peer: &Client, bitfield: &mut [PieceStatus]) -> Result<(), ClientError> {
    if bitfield.len() < client_peer.torrent_file.total_amount_pieces {
        return Err(ClientError::UpdatingBitfield(
            "La longitud del bitfield es incorrecta.".to_string(),
        ));
    }

    if is_any_spare_bit_set(client_peer, bitfield) {
        return Err(ClientError::UpdatingBitfield(
            "Alguno de los bits de repuesto esta seteado en uno.".to_string(),
        ));
    }
    Ok(())
}

fn check_and_truncate_bitfield_according_to_total_amount_of_pieces(
    client_peer: &Client,
    bitfield: &mut Vec<PieceStatus>,
) -> Result<(), ClientError> {
    check_bitfield(client_peer, bitfield)?;
    bitfield.truncate(client_peer.torrent_file.total_amount_pieces);
    Ok(())
}

fn update_peers_data_list(
    peers_data_list: &mut PeersDataList,
    bitfield: Vec<PieceStatus>,
    server_peer_index: usize,
) -> Result<(), ClientError> {
    let peer_data = peers_data_list.data_list.get_mut(server_peer_index);
    match peer_data {
        Some(peer_data) => {
            peer_data.pieces_availability = Some(bitfield);
            Ok(())
        }
        None => Err(ClientError::UpdatingBitfield(
            "No se encontró ningun peer en el indice dado".to_string(),
        )),
    }
}

fn update_peer_bitfield(
    client_peer: &mut Client,
    mut bitfield: Vec<PieceStatus>,
    server_peer_index: usize,
) -> Result<(), ClientError> {
    check_and_truncate_bitfield_according_to_total_amount_of_pieces(client_peer, &mut bitfield)?;
    match &mut client_peer.peers_data_list {
        Some(peers_data_list) => {
            update_peers_data_list(peers_data_list, bitfield, server_peer_index)
        }
        None => Err(ClientError::UpdatingBitfield(
            "Acceso invalido a la lista de peers".to_string(),
        )),
    }
}

//LOOK FOR PIECES INDEX
fn server_peer_has_a_valid_and_available_piece_on_position(
    client_peer: &Client,
    server_peer_index: usize,
    position: usize,
) -> bool {
    if let Some(peers_data_list) = &client_peer.peers_data_list {
        if let Some(server_peer_data) = peers_data_list.data_list.get(server_peer_index) {
            if let Some(pieces_availability) = &server_peer_data.pieces_availability {
                return pieces_availability[position] == PieceStatus::ValidAndAvailablePiece;
            }
        }
    }

    false
}

fn look_for_a_missing_piece_index(client_peer: &Client, server_peer_index: usize) -> Option<usize> {
    for (piece_index, _piece_status) in client_peer
        .client_data
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

// UPDATING FIELDS
fn update_am_interested_field(client_peer: &mut Client, server_peer_index: usize, new_value: bool) {
    if let Some(peers_data_list) = &mut client_peer.peers_data_list {
        if let Some(server_peer_data) = peers_data_list.data_list.get_mut(server_peer_index) {
            server_peer_data.am_interested = new_value;
        }
    }
}

fn update_am_choking_field(client_peer: &mut Client, server_peer_index: usize, new_value: bool) {
    if let Some(peers_data_list) = &mut client_peer.peers_data_list {
        if let Some(server_peer_data) = peers_data_list.data_list.get_mut(server_peer_index) {
            server_peer_data.am_chocking = new_value;
        }
    }
}

fn update_peer_choking_field(client_peer: &mut Client, server_peer_index: usize, new_value: bool) {
    if let Some(peers_data_list) = &mut client_peer.peers_data_list {
        if let Some(server_peer_data) = peers_data_list.data_list.get_mut(server_peer_index) {
            server_peer_data.peer_choking = new_value;
        }
    }
}

// ASK FOR INFORMATION
fn am_choking(client_peer: &Client, server_peer_index: usize) -> bool {
    if let Some(peers_data_list) = &client_peer.peers_data_list {
        if let Some(server_peer_data) = peers_data_list.data_list.get(server_peer_index) {
            return server_peer_data.am_chocking;
        }
    }
    true
}

// Notas
// Miguel y Luciano: En el google docs se tiene que antes de establecer conexion con peers se debe ver como viene la
//  lista de peers segun clave compact de la respuesta del tracker.
//  Esto en realidad es, justamente, responsabilidad de lo que se encargue de recibir la info del tracker.
//
// Miguel: Estuve releyendo el proceso y esta funcion en realidad seria algo como la entrada principal a toda la logica de conexion.
//  O sea, necesitamos una funcion (esta misma) que deberia establecer y manejar la conexion con todos los peers (o con los necesarios)
//  y luego hacer algo asi como llamar a una funcion que se encargue de hacer todo el protocolo de leecher con distintos peers en threads.
//
// Miguel: Nota aparte nada que ver (de pensamiento en threads). Se me ocurrió por ejemplo, en los threads individuales que van corriendo esta funcion por cada peer, usar channels para comunicarle al thread padre que ya llegó al punto en el que tiene el bitfield de cada peer, de tal forma que dicho thread padre se encargue de analizar los datos de todos los bitfields y de ahi poder responderle a los threads hijos con exactamente cuales bloques se le deben Requestear a cada peer.

//FUNCION PRINCIPAL
pub fn handle_client_communication(
    client_peer: &mut Client,
    server_peer_index: usize,
) -> Result<(), ClientError> {
    // (MIGUEL) VER NOTA DE client_struct.rs; parte de Error

    //Luciano: NUEVA IDEA se me ocurre tener dos estructuras: MsgSender y MsgReceiver al que le pasas una unica vez el cliente y el stream y listo, despeus solo haces msg_sender.send_handshake() o msg_receiver.receive_msg()

    //CONEXION CON UN PEER
    let mut client_stream =
        start_connection_with_a_peers(&client_peer.tracker_response.peers, server_peer_index)?;

    //ENVIO HANDSHAKE
    send_handshake(client_peer, &mut client_stream)?;

    //RECIBO HANDSHAKE
    let received_handshake = receive_handshake(&mut client_stream)?;
    check_and_save_handshake_data(client_peer, received_handshake, server_peer_index)?;

    //RECIBO UN MENSAJE
    let received_msg = receive_message(&mut client_stream)?;
    match received_msg {
        P2PMessage::KeepAlive => {
            // deberia extender el tiempo antes de cortar la conexion
            Ok(())
        }
        P2PMessage::Choke => {
            //capaz esto podria devolver un Result<>
            update_am_choking_field(client_peer, server_peer_index, true);
            Ok(())
        }
        P2PMessage::Unchoke => {
            //capaz esto podria devolver un Result<>
            update_am_choking_field(client_peer, server_peer_index, false);
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
        P2PMessage::Have { piece_index: _ } => {
            //
            Ok(())
        }
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
            //aca si hago porque es el bloque recibido
            Ok(())
        }
        P2PMessage::Cancel {
            piece_index: _,
            beginning_byte_index: _,
            amount_of_bytes: _,
        } => {
            //
            Ok(())
        }
        P2PMessage::Port { listen_port: _ } => {
            //no se que deberia hacer con esto pero momentaneamente nada
            Ok(())
        }
        _ => Ok(()),
    }?;

    let _piece_index = match look_for_a_missing_piece_index(client_peer, server_peer_index) {
        Some(piece_index) => {
            update_am_interested_field(client_peer, server_peer_index, true);
            piece_index
        }
        None => {
            update_am_interested_field(client_peer, server_peer_index, false);
            return Ok(()); //corto la conexion pero sin error porque solamente no me interesan sus piezas
        }
    };

    if !am_choking(client_peer, server_peer_index) {
        send_interested(client_peer, &mut client_stream)?;
    } else {
        //send_request()
    }
    //Si me tiene choke -> le mando el interested
    //Si no me tiene choke -> le mando request
    Ok(())
}

// #[cfg(test)]
// mod test_msg_logic_control {
//     use super::*;
//     use std::error::Error;
//     use std::sync::mpsc;

//     use crate::torrent::data::client_data::{ClientData, ClientState};
//     use crate::torrent::data::torrent_file_data::TorrentFileData;
//     use crate::torrent::data::tracker_response_data::{
//         TrackerResponseData, TrackerResponsePeerData,
//     };
//     use crate::torrent::parsers::p2p::constants::PSTR_STRING_HANDSHAKE;
//     use crate::torrent::parsers::{p2p, p2p::message::P2PMessage};
//     use std::io::Write;
//     use std::net::{SocketAddr, TcpListener};
//     use std::str::FromStr;
//     use std::thread;

//     fn create_default_client_peer_with_no_server_peers() -> Result<Client, Box<dyn Error>> {
//         let server_peer = TrackerResponsePeerData {
//             peer_id: Some(DEFAULT_SERVER_PEER_ID.to_string()),
//             peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
//         };

//         let tracker_response = TrackerResponseData {
//             interval: 0,
//             tracker_id: DEFAULT_TRACKER_ID.to_string(),
//             complete: 1,
//             incomplete: 0,
//             peers: vec![server_peer],
//         };
//         let client_data = ClientData {
//             peer_id: DEFAULT_CLIENT_PEER_ID.to_string(),
//             info_hash: DEFAULT_INFO_HASH.to_vec(),
//             uploaded: 0,
//             downloaded: 0,
//             left: 16,
//             event: ClientState::Started,
//             pieces_availability: vec![PieceStatus::MissingPiece],
//         };
//         let torrent_file = TorrentFileData {
//             piece_lenght: 16,
//             total_amount_pieces: 1,
//         };
//         Ok(Client {
//             client_data,
//             torrent_file,
//             tracker_response,
//             peers_data_list: None,
//         })
//     }

//     fn create_default_client_peer_with_a_server_peer_that_has_just_one_valid_piece(
//     ) -> Result<Client, Box<dyn Error>> {
//         let server_peer = TrackerResponsePeerData {
//             peer_id: Some(DEFAULT_SERVER_PEER_ID.to_string()),
//             peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
//         };
//         let tracker_response = TrackerResponseData {
//             interval: 0,
//             tracker_id: DEFAULT_TRACKER_ID.to_string(),
//             complete: 1,
//             incomplete: 0,
//             peers: vec![server_peer],
//         };
//         let client_data = ClientData {
//             peer_id: DEFAULT_CLIENT_PEER_ID.to_string(),
//             info_hash: DEFAULT_INFO_HASH.to_vec(),
//             uploaded: 0,
//             downloaded: 0,
//             left: 16,
//             event: ClientState::Started,
//             pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
//         };
//         let torrent_file = TorrentFileData {
//             piece_lenght: 16,
//             total_amount_pieces: 2,
//         };
//         let server_peer_data = PeerData {
//             peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//             pieces_availability: Some(vec![
//                 PieceStatus::MissingPiece,
//                 PieceStatus::ValidAndAvailablePiece,
//             ]),
//             am_interested: false,
//             am_chocking: true,
//             peer_choking: true,
//         };
//         let peers_data_list = Some(PeersDataList {
//             total_amount_of_peers: 1,
//             data_list: vec![server_peer_data],
//         });
//         Ok(Client {
//             client_data,
//             torrent_file,
//             tracker_response,
//             peers_data_list,
//         })
//     }

//     fn create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces(
//     ) -> Result<Client, Box<dyn Error>> {
//         let server_peer = TrackerResponsePeerData {
//             peer_id: Some(DEFAULT_SERVER_PEER_ID.to_string()),
//             peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
//         };
//         let tracker_response = TrackerResponseData {
//             interval: 0,
//             tracker_id: DEFAULT_TRACKER_ID.to_string(),
//             complete: 1,
//             incomplete: 0,
//             peers: vec![server_peer],
//         };
//         let client_data = ClientData {
//             peer_id: DEFAULT_CLIENT_PEER_ID.to_string(),
//             info_hash: DEFAULT_INFO_HASH.to_vec(),
//             uploaded: 0,
//             downloaded: 0,
//             left: 16,
//             event: ClientState::Started,
//             pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
//         };
//         let torrent_file = TorrentFileData {
//             piece_lenght: 16,
//             total_amount_pieces: 2,
//         };
//         let server_peer_data = PeerData {
//             peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//             pieces_availability: Some(vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece]),
//             am_interested: false,
//             am_chocking: true,
//             peer_choking: true,
//         };
//         let peers_data_list = Some(PeersDataList {
//             total_amount_of_peers: 1,
//             data_list: vec![server_peer_data],
//         });
//         Ok(Client {
//             client_data,
//             torrent_file,
//             tracker_response,
//             peers_data_list,
//         })
//     }

//     mod test_check_and_save_handshake_data {
//         use super::*;

//         #[test]
//         fn receive_a_message_that_is_not_a_handshake_error() -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             let message = P2PMessage::KeepAlive;

//             assert!(
//                 check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
//                     .is_err()
//             );

//             Ok(())
//         }

//         #[test]
//         fn receive_a_handshake_with_an_incorrect_protocol_str_error() -> Result<(), Box<dyn Error>>
//         {
//             let server_piece_index = 0;
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             let message = P2PMessage::Handshake {
//                 protocol_str: "VitTorrent protocol".to_string(),
//                 info_hash: DEFAULT_INFO_HASH.to_vec(),
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//             };

//             assert!(
//                 check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
//                     .is_err()
//             );

//             Ok(())
//         }

//         #[test]
//         fn receive_a_handshake_with_an_incorrect_info_hash_error() -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             let message = P2PMessage::Handshake {
//                 protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//                 info_hash: [1; 20].to_vec(),
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//             };

//             assert!(
//                 check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
//                     .is_err()
//             );

//             Ok(())
//         }

//         #[test]
//         fn receive_a_handshake_with_an_incorrect_peer_id_error() -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             let message = P2PMessage::Handshake {
//                 protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//                 info_hash: DEFAULT_INFO_HASH.to_vec(),
//                 peer_id: "-FA0001-000000000002".to_string(),
//             };

//             assert!(
//                 check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
//                     .is_err()
//             );

//             Ok(())
//         }

//         #[test]
//         fn client_that_has_no_peer_ids_to_check_receive_a_valid_handshake_ok(
//         ) -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;

//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             //MODIFICO EL CLIENTE PARA QUE NO TENGA LOS PEER_ID DE LOS SERVER PEER
//             client_peer.tracker_response.peers = vec![TrackerResponsePeerData {
//                 peer_id: None,
//                 peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
//             }];

//             let message = P2PMessage::Handshake {
//                 protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//                 info_hash: DEFAULT_INFO_HASH.to_vec(),
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//             };
//             let expected_peer_data = PeerData {
//                 pieces_availability: None,
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//                 am_interested: false,
//                 am_chocking: true,
//                 peer_choking: true,
//             };

//             assert!(
//                 check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
//                     .is_ok()
//             );
//             assert!(client_peer.peers_data_list.is_some());
//             if let Some(peer_data_list) = client_peer.peers_data_list {
//                 assert_eq!(vec![expected_peer_data], peer_data_list.data_list);
//                 assert_eq!(1, peer_data_list.total_amount_of_peers);
//             }

//             Ok(())
//         }

//         #[test]
//         fn client_that_has_peer_ids_to_check_receive_a_valid_handshake_ok(
//         ) -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             let message = P2PMessage::Handshake {
//                 protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//                 info_hash: DEFAULT_INFO_HASH.to_vec(),
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//             };
//             let expected_peer_data = PeerData {
//                 pieces_availability: None,
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//                 am_interested: false,
//                 am_chocking: true,
//                 peer_choking: true,
//             };

//             assert!(
//                 check_and_save_handshake_data(&mut client_peer, message, server_piece_index)
//                     .is_ok()
//             );

//             assert!(client_peer.peers_data_list.is_some());
//             if let Some(peer_data_list) = client_peer.peers_data_list {
//                 assert_eq!(vec![expected_peer_data], peer_data_list.data_list);
//                 assert_eq!(1, peer_data_list.total_amount_of_peers);
//             }

//             Ok(())
//         }
//     }

//     mod test_update_peer_bitfield {
//         use super::*;

//         #[test]
//         fn update_peer_bitfield_with_less_pieces_error() -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             let bitfield = vec![];

//             let peer_data = PeerData {
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//                 pieces_availability: None,
//                 am_interested: false,
//                 am_chocking: true,
//                 peer_choking: true,
//             };
//             let peer_data_list = PeersDataList {
//                 total_amount_of_peers: 1,
//                 data_list: vec![peer_data],
//             };
//             client_peer.peers_data_list = Some(peer_data_list);

//             assert!(update_peer_bitfield(&mut client_peer, bitfield, server_piece_index).is_err());

//             if let Some(peer_data_list) = client_peer.peers_data_list {
//                 if let Some(server_peer_data) = peer_data_list.data_list.get(server_piece_index) {
//                     assert!(server_peer_data.pieces_availability.is_none())
//                 }
//             }

//             Ok(())
//         }
//         #[test]
//         fn update_peer_bitfield_with_more_pieces_and_spare_bits_set_error(
//         ) -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             let bitfield = vec![
//                 PieceStatus::ValidAndAvailablePiece,
//                 PieceStatus::MissingPiece,
//                 PieceStatus::ValidAndAvailablePiece,
//                 PieceStatus::ValidAndAvailablePiece,
//             ];

//             let peer_data = PeerData {
//                 pieces_availability: None,
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//                 am_interested: false,
//                 am_chocking: true,
//                 peer_choking: true,
//             };
//             let peer_data_list = PeersDataList {
//                 total_amount_of_peers: 1,
//                 data_list: vec![peer_data],
//             };
//             client_peer.peers_data_list = Some(peer_data_list);

//             assert!(update_peer_bitfield(&mut client_peer, bitfield, server_piece_index).is_err());

//             if let Some(peer_data_list) = client_peer.peers_data_list {
//                 if let Some(server_peer_data) = peer_data_list.data_list.get(server_piece_index) {
//                     assert!(server_peer_data.pieces_availability.is_none())
//                 }
//             }

//             Ok(())
//         }
//         #[test]
//         fn update_peer_bitfield_with_the_correct_amount_of_pieces_ok() -> Result<(), Box<dyn Error>>
//         {
//             let server_piece_index = 0;
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             let bitfield = vec![PieceStatus::ValidAndAvailablePiece];

//             let peer_data = PeerData {
//                 pieces_availability: None,
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//                 am_interested: false,
//                 am_chocking: true,
//                 peer_choking: true,
//             };
//             let peer_data_list = PeersDataList {
//                 total_amount_of_peers: 1,
//                 data_list: vec![peer_data],
//             };
//             client_peer.peers_data_list = Some(peer_data_list);

//             assert!(update_peer_bitfield(&mut client_peer, bitfield, server_piece_index).is_ok());

//             if let Some(peer_data_list) = client_peer.peers_data_list {
//                 if let Some(server_peer_data) = peer_data_list.data_list.get(server_piece_index) {
//                     assert!(server_peer_data.pieces_availability.is_some());
//                     if let Some(piece_availability) = &server_peer_data.pieces_availability {
//                         assert_eq!(
//                             vec![PieceStatus::ValidAndAvailablePiece],
//                             *piece_availability
//                         )
//                     }
//                 }
//             }
//             Ok(())
//         }
//         #[test]
//         fn update_peer_bitfield_with_more_pieces_and_spare_bits_not_set_ok(
//         ) -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;
//             let bitfield = vec![
//                 PieceStatus::ValidAndAvailablePiece,
//                 PieceStatus::MissingPiece,
//                 PieceStatus::MissingPiece,
//                 PieceStatus::MissingPiece,
//             ];

//             let peer_data = PeerData {
//                 pieces_availability: None,
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//                 am_interested: false,
//                 am_chocking: true,
//                 peer_choking: true,
//             };
//             let peer_data_list = PeersDataList {
//                 total_amount_of_peers: 1,
//                 data_list: vec![peer_data],
//             };
//             client_peer.peers_data_list = Some(peer_data_list);

//             assert!(update_peer_bitfield(&mut client_peer, bitfield, server_piece_index).is_ok());

//             if let Some(peer_data_list) = client_peer.peers_data_list {
//                 if let Some(server_peer_data) = peer_data_list.data_list.get(server_piece_index) {
//                     assert!(server_peer_data.pieces_availability.is_some());
//                     if let Some(piece_availability) = &server_peer_data.pieces_availability {
//                         assert_eq!(
//                             vec![PieceStatus::ValidAndAvailablePiece],
//                             *piece_availability
//                         )
//                     }
//                 }
//             }
//             Ok(())
//         }
//     }

//     mod test_look_for_a_missing_piece_index {
//         use super::*;

//         #[test]
//         fn the_server_peer_has_a_valid_and_available_piece_in_the_position_one(
//         ) -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;
//             let client_peer =
//                 create_default_client_peer_with_a_server_peer_that_has_just_one_valid_piece()?;

//             assert_eq!(
//                 Some(1),
//                 look_for_a_missing_piece_index(&client_peer, server_piece_index)
//             );
//             Ok(())
//         }

//         #[test]
//         fn the_server_peer_has_no_pieces() -> Result<(), Box<dyn Error>> {
//             let server_piece_index = 0;
//             let client_peer =
//                 create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces()?;

//             assert_eq!(
//                 None,
//                 look_for_a_missing_piece_index(&client_peer, server_piece_index)
//             );
//             Ok(())
//         }
//     }

//     mod test_handle_client_communication {
//         use super::*;
//         #[test]
//         fn the_client_send_a_handshake_when_starts_ok() -> Result<(), Box<dyn Error>> {
//             // ABRO LA CONEXION
//             let listener = TcpListener::bind(DEFAULT_ADDR)?;

//             // CREACION DE UN CLIENTE PEER
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;

//             assert!(handle_client_communication(&mut client_peer, 0).is_err()); //esto tiene que fallar si o si porque no tuvimos una comunucacion exitosa con ese peer.

//             //RECIBO LO QUE ME DEBERIA HABER MANDADO EL CLIENTE
//             let (mut server_stream, _addr) = listener.accept()?;
//             let received_message = receive_handshake(&mut server_stream)?;

//             assert_eq!(
//                 P2PMessage::Handshake {
//                     protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//                     info_hash: DEFAULT_INFO_HASH.to_vec(),
//                     peer_id: DEFAULT_CLIENT_PEER_ID.to_string(), //no deberia comparar con el peed id para ver si son iguales porque ese valor siempre va a ser aleatorio.

//                                                                  // Miguel: No no pero no va a ser aleatorio siempre. Tipo vos vas a generar una vez ese peer id y eso te lo guardas, no vas a ir cambiandolo. Es como la identidad de ese peer, con eso verificas que si sea el mismo de antes.
//                 },
//                 received_message
//             );
//             Ok(())
//         }

//         #[test]
//         fn the_server_peer_is_added_into_the_peers_data_list_of_the_client_peer_ok(
//         ) -> Result<(), Box<dyn Error>> {
//             // ABRO LA CONEXION
//             let listener = TcpListener::bind(DEFAULT_ADDR)?;

//             // CREACION DE UN CLIENTE PEER
//             let mut client_peer = create_default_client_peer_with_no_server_peers()?;

//             //THREAD SECUNDARIO PARA EL CLIENTE
//             let (tx, rx) = mpsc::channel();
//             let handle = thread::spawn(move || {
//                 //HANDLEO COMUNICACION
//                 let _result = handle_client_communication(&mut client_peer, 0); //sacar unwrap
//                 tx.send(client_peer).unwrap();
//             });

//             //RECIBO LO QUE ME DEBERIA HABER MANDADO EL CLIENTE
//             let (mut server_stream, _addr) = listener.accept()?;
//             let received_message = receive_handshake(&mut server_stream)?;

//             assert_eq!(
//                 P2PMessage::Handshake {
//                     protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//                     info_hash: DEFAULT_INFO_HASH.to_vec(),
//                     peer_id: DEFAULT_CLIENT_PEER_ID.to_string(), //no deberia comparar con el peed id para ver si son iguales porque ese valor siempre va a ser aleatorio
//                 },
//                 received_message
//             );

//             //ENVIO UN HANDSHAKE DE RESPUESTA
//             let server_handshake = P2PMessage::Handshake {
//                 protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//                 info_hash: DEFAULT_INFO_HASH.to_vec(),
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//             };
//             let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake)?;
//             server_stream.write_all(&server_handshake_bytes)?;

//             //VEO QUE LE HAYA LLEGADO Y QUE ADEMAS LO ACEPTE
//             let client_peer = rx.recv()?;
//             assert!(client_peer.peers_data_list.is_some());
//             match client_peer.peers_data_list {
//                 Some(peers_data_list) => {
//                     assert_eq!(1, peers_data_list.total_amount_of_peers);
//                     assert_eq!(
//                         DEFAULT_SERVER_PEER_ID.to_string(),
//                         peers_data_list.data_list[0].peer_id
//                     );
//                     assert!(peers_data_list.data_list[0].pieces_availability.is_none())
//                 }
//                 None => (),
//             }

//             let _joined = handle.join(); //ver que hacer con ese error

//             Ok(())
//         }
//     }
// }
