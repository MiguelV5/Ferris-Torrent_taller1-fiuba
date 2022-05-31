use core::fmt;
use std::error::Error;
use std::io::ErrorKind;
use std::sync::mpsc;

use fa_torrent::torrent::client::client_struct::Client;
use fa_torrent::torrent::client::peers_comunication::msg_logic_control;
use fa_torrent::torrent::client::peers_comunication::msg_receiver;
use fa_torrent::torrent::data::data_of_download::{DataOfDownload, StateOfDownload};
use fa_torrent::torrent::data::torrent_file_data::TorrentFileData;
use fa_torrent::torrent::data::tracker_response_data::{
    PeerDataFromTrackerResponse, TrackerResponseData,
};
use fa_torrent::torrent::parsers::p2p::constants::PSTR_STRING_HANDSHAKE;
use fa_torrent::torrent::parsers::p2p::message::PieceStatus;
use fa_torrent::torrent::parsers::{p2p, p2p::message::P2PMessage};
use std::io::Write;
use std::net::{SocketAddr, TcpListener};
use std::str::FromStr;
use std::thread;

const LOCALHOST: &str = "127.0.0.1";
const STARTING_PORT: u16 = 8080;
const MAX_TESTING_PORT: u16 = 9080;
// pub const DEFAULT_ADDR_1: &str = "127.0.0.1:7878";
// pub const DEFAULT_ADDR_2: &str = "127.0.0.1:7979";
// pub const DEFAULT_ADDR_3: &str = "127.0.0.1:8080";
pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";
pub const DEFAULT_TRACKER_ID: &str = "Tracker ID";
pub const DEFAULT_TRACKER_MAIN: &str = "tracker_main.com";
pub const DEFAULT_INFO_HASH: [u8; 20] = [0; 20];

#[derive(PartialEq, Debug, Clone)]
pub enum TestingError {
    ClientPeerFieldsInvalidAccess(String),
}

impl fmt::Display for TestingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for TestingError {}

//
// FUNCIONES AUXILIARES:
//
fn create_default_client_peer(peer_address: SocketAddr) -> Result<Client, Box<dyn Error>> {
    let server_peer = PeerDataFromTrackerResponse {
        peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
        peer_address,
    };

    let tracker_response = TrackerResponseData {
        interval: 0,
        //tracker_id: DEFAULT_TRACKER_ID.to_string(),
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
        name: "nombre.txt".to_string(),
        url_tracker_main: "tracker_main.com".to_string(),
        url_tracker_list: vec![],
        info_hash: DEFAULT_INFO_HASH.to_vec(),
        pieces: vec![],
        piece_length: 16,
        path: vec![],
        total_amount_pieces: 1,
        total_size: 16,
    };
    Ok(Client {
        peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
        info_hash: DEFAULT_INFO_HASH.to_vec(),

        data_of_download,
        torrent_file,
        tracker_response: Some(tracker_response),
        list_of_peers_data_for_communication: None,
    })
}

#[derive(PartialEq, Debug)]
enum PortBindingError {
    ReachedMaxPortWithoutFindingAnAvailableOne,
}

impl fmt::Display for PortBindingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for PortBindingError {}

fn update_port(current_port: u16) -> Result<u16, PortBindingError> {
    let mut new_port: u16 = current_port;
    if current_port >= MAX_TESTING_PORT {
        Err(PortBindingError::ReachedMaxPortWithoutFindingAnAvailableOne)
    } else {
        new_port += 1;
        Ok(new_port)
    }
}

// Busca bindear un listener mientras que el error sea por causa de una direccion que ya está en uso.
fn try_bind_listener(first_port: u16) -> Result<(TcpListener, String), Box<dyn Error>> {
    let mut listener = TcpListener::bind(format!("{}:{}", LOCALHOST, first_port));

    let mut current_port = first_port;

    while let Err(bind_err) = listener {
        if bind_err.kind() != ErrorKind::AddrInUse {
            return Err(Box::new(bind_err));
        } else {
            current_port = update_port(current_port)?;
            listener = TcpListener::bind(format!("{}:{}", LOCALHOST, current_port));
        }
    }
    let resulting_listener = listener?; // SI BIEN TIENE ?; ACÁ NUNCA VA A SER UN ERROR

    Ok((
        resulting_listener,
        format!("{}:{}", LOCALHOST, current_port),
    ))
}

//
// TESTS:
//
#[test]
fn the_client_send_a_handshake_when_starts_ok() -> Result<(), Box<dyn Error>> {
    // ABRO LA CONEXION
    let (listener, address) = try_bind_listener(STARTING_PORT)?;

    // CREACION DE UN CLIENTE PEER
    let mut client_peer = create_default_client_peer(SocketAddr::from_str(&address)?)?;

    assert!(msg_logic_control::interact_with_single_peer(&mut client_peer, 0).is_err());

    //RECIBO LO QUE ME DEBERIA HABER MANDADO EL CLIENTE
    let (mut server_stream, _addr) = listener.accept()?;
    let received_message = msg_receiver::receive_handshake(&mut server_stream)?;

    assert_eq!(
        P2PMessage::Handshake {
            protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
        },
        received_message
    );
    Ok(())
}

#[test]
fn client_peer_receives_a_handshake_ok() -> Result<(), Box<dyn Error>> {
    // ABRO LA CONEXION
    let (listener, address) = try_bind_listener(STARTING_PORT)?;

    // CREACION DE UN CLIENTE PEER
    let mut client_peer = create_default_client_peer(SocketAddr::from_str(&address)?)?;

    //THREAD SECUNDARIO PARA EL CLIENTE
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        //HANDLEO COMUNICACION
        let _result = msg_logic_control::interact_with_single_peer(&mut client_peer, 0);
        tx.send(client_peer).unwrap();
    });

    //RECIBO LO QUE ME DEBERIA HABER MANDADO EL CLIENTE
    let (mut server_stream, _addr) = listener.accept()?;
    let received_message = msg_receiver::receive_handshake(&mut server_stream)?;

    assert_eq!(
        P2PMessage::Handshake {
            protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
        },
        received_message
    );

    //ENVIO UN HANDSHAKE DE RESPUESTA
    let server_handshake = P2PMessage::Handshake {
        protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
        info_hash: DEFAULT_INFO_HASH.to_vec(),
        peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
    };
    let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake)?;
    server_stream.write_all(&server_handshake_bytes)?;

    //VEO QUE LE HAYA LLEGADO Y QUE ADEMAS LO ACEPTE
    let client_peer = rx.recv()?;
    if let Some(list_of_peers_data_for_communication) =
        client_peer.list_of_peers_data_for_communication
    {
        assert_eq!(1, list_of_peers_data_for_communication.len());
        let expected_id: Vec<u8> = DEFAULT_SERVER_PEER_ID.bytes().collect();
        assert_eq!(expected_id, list_of_peers_data_for_communication[0].peer_id);
        assert!(list_of_peers_data_for_communication[0]
            .pieces_availability
            .is_none());

        let _joined = handle.join(); //ver que hacer con ese error
        return Ok(());
    }

    Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
        "Couldn`t access to client peer fields.".to_string(),
    )))
}

#[test]
fn client_peer_interact_with_a_peer_ok() -> Result<(), Box<dyn Error>> {
    // ABRO LA CONEXION
    let (listener, address) = try_bind_listener(STARTING_PORT)?;

    // CREACION DE UN CLIENTE PEER
    let mut client_peer = create_default_client_peer(SocketAddr::from_str(&address)?)?;

    //THREAD SECUNDARIO PARA EL CLIENTE
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        //HANDLEO COMUNICACION
        let _result = msg_logic_control::interact_with_single_peer(&mut client_peer, 0);
        tx.send(client_peer).unwrap();
    });

    let (mut server_stream, _addr) = listener.accept()?;

    //SERVER PEER RECIBE UN HANDSHAKE
    let received_message = msg_receiver::receive_handshake(&mut server_stream)?;
    assert_eq!(
        P2PMessage::Handshake {
            protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
        },
        received_message
    );

    //SERVER PEER ENVIA UN HANDSHAKE DE RESPUESTA
    let server_handshake = P2PMessage::Handshake {
        protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
        info_hash: DEFAULT_INFO_HASH.to_vec(),
        peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
    };
    let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake)?;
    server_stream.write_all(&server_handshake_bytes)?;

    //SERVER PEER ENVIA UN BITFIELD
    let server_msg = P2PMessage::Bitfield {
        bitfield: vec![PieceStatus::ValidAndAvailablePiece],
    };
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream.write_all(&server_msg_bytes)?;

    //SERVER PEER RECIVE UN INTERESTED
    let received_message = msg_receiver::receive_message(&mut server_stream)?;
    assert_eq!(P2PMessage::Interested, received_message);

    //SERVER PEER ENVIA UN UNCHOKE
    let server_msg = P2PMessage::Unchoke;
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream.write_all(&server_msg_bytes)?;

    //SERVER PEER RECIBE UN REQUEST
    let received_message = msg_receiver::receive_message(&mut server_stream)?;
    assert_eq!(
        P2PMessage::Request {
            piece_index: 0,
            beginning_byte_index: 0,
            amount_of_bytes: 16
        },
        received_message
    );

    //SERVER PEER ENVIA UN BLOQUE QUE CORRESPONDE A LA PIEZA ENTERA
    let server_msg = P2PMessage::Piece {
        piece_index: 0,
        beginning_byte_index: 0,
        block: [10; 16].to_vec(),
    };
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream.write_all(&server_msg_bytes)?;

    //VEO QUE LE HAYA LLEGADO Y QUE ADEMAS LO ACEPTE
    let client_peer = rx.recv()?;
    assert!(client_peer.list_of_peers_data_for_communication.is_some());
    if let Some(list_of_peers_data_for_communication) =
        client_peer.list_of_peers_data_for_communication
    {
        assert_eq!(1, list_of_peers_data_for_communication.len());
        let expected_id: Vec<u8> = DEFAULT_SERVER_PEER_ID.bytes().collect();
        assert_eq!(expected_id, list_of_peers_data_for_communication[0].peer_id);

        if let Some(pieces_availability) =
            &list_of_peers_data_for_communication[0].pieces_availability
        {
            assert_eq!(
                vec![PieceStatus::ValidAndAvailablePiece,],
                *pieces_availability
            );
            let _joined = handle.join(); //ver que hacer con ese error
            return Ok(());
        }
    }

    Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
        "Couldn`t access to client peer fields.".to_string(),
    )))
}
