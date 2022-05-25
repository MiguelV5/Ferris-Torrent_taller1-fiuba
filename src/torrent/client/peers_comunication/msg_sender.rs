use crate::torrent::client::client_struct::*;
use crate::torrent::parsers::{
    p2p, p2p::constants::PSTR_STRING_HANDSHAKE, p2p::message::P2PMessage,
};
use std::io::Write;
use std::net::TcpStream;

pub fn send_handshake(client: &Client, stream: &mut TcpStream) -> Result<(), ClientError> {
    let handshake_bytes = p2p::encoder::to_bytes(P2PMessage::Handshake {
        protocol_str: PSTR_STRING_HANDSHAKE.to_string(), //todos estos campos estan en el cliente ya
        info_hash: client.client_data.info_hash.clone(),
        peer_id: client.client_data.peer_id.clone(),
    })
    .map_err(|error| ClientError::ConectingWithPeer(format!("{:?}", error)))?;

    stream
        .write_all(&handshake_bytes)
        .map_err(|error| ClientError::ConectingWithPeer(format!("{:?}", error)))?; //En realidad es otro tipo de error, revisar.
    Ok(())
}

fn send_msg(
    _client: &Client,
    stream: &mut TcpStream,
    msg_variant: P2PMessage,
) -> Result<(), ClientError> {
    let msg_bytes = p2p::encoder::to_bytes(msg_variant)
        .map_err(|error| ClientError::ConectingWithPeer(format!("{:?}", error)))?;

    stream
        .write_all(&msg_bytes)
        .map_err(|error| ClientError::ConectingWithPeer(format!("{:?}", error)))?; //En realidad es otro tipo de error, revisar.
    Ok(())
}

pub fn send_keep_alive(_client: &Client, stream: &mut TcpStream) -> Result<(), ClientError> {
    send_msg(_client, stream, P2PMessage::KeepAlive)
}

pub fn send_choke(_client: &Client, stream: &mut TcpStream) -> Result<(), ClientError> {
    send_msg(_client, stream, P2PMessage::Choke)
}

pub fn send_unchoke(_client: &Client, stream: &mut TcpStream) -> Result<(), ClientError> {
    send_msg(_client, stream, P2PMessage::Unchoke)
}

pub fn send_interested(_client: &Client, stream: &mut TcpStream) -> Result<(), ClientError> {
    send_msg(_client, stream, P2PMessage::Interested)
}

pub fn send_not_interested(_client: &Client, stream: &mut TcpStream) -> Result<(), ClientError> {
    send_msg(_client, stream, P2PMessage::NotInterested)
}

pub fn send_have(
    _client: &Client,
    stream: &mut TcpStream,
    completed_piece_index: u32,
) -> Result<(), ClientError> {
    let have_msg = P2PMessage::Have {
        piece_index: completed_piece_index, // Revisar, pero creo que esta info no se tiene en el cliente
    };
    send_msg(_client, stream, have_msg)
}

// pub fn send_bitfield(_client: &Client, stream: &mut TcpStream) -> Result<(), ClientError> {
//     let availavility_of_pieces = _client.peers_data_list.data_list[0]
//         .pieces_availability
//         .clone(); // si se descomenta el campo de la estructura Client hay que modificar msg_logic_control

//     let bitfield_msg = P2PMessage::Bitfield {
//         bitfield: availavility_of_pieces,
//     };
//     send_msg(_client, stream, bitfield_msg)
// }

// #[cfg(test)]
// mod test_msg_sender {
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
//     mod test_send_handshake {
//         use crate::torrent::{
//             client::peers_comunication::msg_logic_control::{
//                 DEFAULT_ADDR, DEFAULT_CLIENT_PEER_ID, DEFAULT_INFO_HASH, DEFAULT_SERVER_PEER_ID,
//                 DEFAULT_TRACKER_ID,
//             },
//             data::peers_data::{PeerData, PeersDataList},
//             parsers::p2p::message::PieceStatus,
//         };

//         use super::*;

//         fn create_default_client_peer_with_a_server_peer_that_has_one_piece(
//         ) -> Result<Client, Box<dyn Error>> {
//             let server_peer = TrackerResponsePeerData {
//                 peer_id: Some(DEFAULT_SERVER_PEER_ID.to_string()),
//                 peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
//             };
//             let tracker_response = TrackerResponseData {
//                 interval: 0,
//                 tracker_id: DEFAULT_TRACKER_ID.to_string(),
//                 complete: 1,
//                 incomplete: 0,
//                 peers: vec![server_peer],
//             };
//             let client_data = ClientData {
//                 peer_id: DEFAULT_CLIENT_PEER_ID.to_string(),
//                 info_hash: DEFAULT_INFO_HASH.to_vec(),
//                 uploaded: 0,
//                 downloaded: 0,
//                 left: 16,
//                 event: ClientState::Started,
//                 pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
//             };
//             let torrent_file = TorrentFileData {
//                 piece_lenght: 16,
//                 total_amount_pieces: 2,
//             };
//             let server_peer_data = PeerData {
//                 peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
//                 pieces_availability: Some(vec![
//                     PieceStatus::MissingPiece,
//                     PieceStatus::ValidAndAvailablePiece,
//                 ]),
//                 am_interested: false,
//                 am_chocking: true,
//                 peer_choking: true,
//             };
//             let peers_data_list = Some(PeersDataList {
//                 total_amount_of_peers: 1,
//                 data_list: vec![server_peer_data],
//             });
//             Ok(Client {
//                 client_data,
//                 torrent_file,
//                 tracker_response,
//                 peers_data_list,
//             })
//         }

//         #[test]
//         fn test1() -> Result<(), Box<dyn Error>> {
//             let listener = TcpListener::bind(DEFAULT_ADDR)?;
//             let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
//             let (mut receptor_stream, _addr) = listener.accept()?;

//             send_handshake();

//             Ok(())
//         }
//     }
// }
