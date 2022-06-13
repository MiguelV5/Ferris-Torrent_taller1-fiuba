use core::fmt;
use fa_torrent::torrent::{
    client::peers_comunication::msg_receiver,
    data::{
        //peer_data_for_communication::PeerDataForP2PCommunication,
        torrent_file_data::{TargetFilesData, TorrentFileData},
        torrent_status::{StateOfDownload, TorrentStatus},
        tracker_response_data::{PeerDataFromTrackerResponse, TrackerResponseData},
    },
    local_peer::LocalPeer,
    parsers::{
        p2p,
        p2p::constants::PSTR_STRING_HANDSHAKE,
        p2p::message::{P2PMessage, PieceStatus},
    },
};
use std::{
    error::Error,
    //fs,
    io::{ErrorKind, Write},
    net::{SocketAddr, TcpListener /*TcpStream*/},
    str::FromStr,
    sync::mpsc,
    thread,
};

const LOCALHOST: &str = "127.0.0.1";
const STARTING_PORT: u16 = 8080;
const MAX_TESTING_PORT: u16 = 9080;

pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";
pub const DEFAULT_TRACKER_ID: &str = "Tracker ID";
pub const DEFAULT_URL_TRACKER_MAIN: &str = "url_tracker_main.com";
pub const DEFAULT_FILE_NAME: &str = "file_name.txt";
pub const DEFAULT_INFO_HASH: [u8; 20] = [0; 20];
pub const DEFAULT_PIECE_LENGHT: usize = 34000;
pub const DEFAULT_AMOUNT_OF_PIECES: usize = 2;
pub const DEFAULT_LAST_PIECE_LENGHT: usize = 6000;

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

//==========================================
// FUNCIONES AUXILIARES:
//
fn create_default_torrent_data(
    torrent_name: &str,
    peer_address: SocketAddr,
) -> Result<(TrackerResponseData, TorrentStatus, TorrentFileData), Box<dyn Error>> {
    let server_peer = PeerDataFromTrackerResponse {
        peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
        peer_address,
    };
    let tracker_response_data = TrackerResponseData {
        interval: 0,
        complete: 1,
        incomplete: 0,
        peers: vec![server_peer],
    };
    let torrent_status = TorrentStatus {
        uploaded: 0,
        downloaded: 0,
        left: 40000,
        event: StateOfDownload::Started,
        pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
    };
    let torrent_file = TorrentFileData {
        target_files_data: TargetFilesData::SingleFile {
            file_name: torrent_name.to_string(),
            file_length: 40000,
            //1º pieza -> 34000 bytes
            //2º pieza ->  6000 bytes
        },
        sha1_pieces: vec![
            202, 159, 46, 61, 51, 96, 133, 196, 50, 186, 241, 152, 214, 63, 131, 198, 120, 150, 83,
            35,
        ],
        url_tracker_main: DEFAULT_URL_TRACKER_MAIN.to_string(),
        url_tracker_list: vec![],
        sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
        piece_length: (DEFAULT_PIECE_LENGHT).try_into()?,
        total_amount_of_pieces: DEFAULT_AMOUNT_OF_PIECES,
        total_length: (DEFAULT_PIECE_LENGHT + DEFAULT_LAST_PIECE_LENGHT).try_into()?,
    };
    Ok((tracker_response_data, torrent_status, torrent_file))
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
pub fn try_bind_listener(first_port: u16) -> Result<(TcpListener, String), Box<dyn Error>> {
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

//==========================================

//
// TESTS:
//
#[test]
fn client_peer_receives_a_handshake_ok() -> Result<(), Box<dyn Error>> {
    // ABRO LA CONEXION
    let (listener, address) = try_bind_listener(STARTING_PORT)?;

    // CREACION DE UN CLIENTE PEER
    let (tracker_response_data, torrent_status, torrent_file_data) =
        create_default_torrent_data("test_handshake_ok.txt", SocketAddr::from_str(&address)?)?;

    //THREAD SECUNDARIO PARA EL CLIENTE
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        //HANDLEO COMUNICACION
        let local_peer =
            LocalPeer::start_communication(&torrent_file_data, &tracker_response_data, 0).unwrap();
        tx.send((local_peer, torrent_status))
    });

    //SERVER PEER RECIBE UN HANDSHAKE
    let (mut server_stream, _addr) = listener.accept()?;
    let received_handshake = msg_receiver::receive_handshake(&mut server_stream)?;

    //ENVIO UN HANDSHAKE DE RESPUESTA
    let server_handshake = P2PMessage::Handshake {
        protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
        info_hash: DEFAULT_INFO_HASH.to_vec(),
        peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
    };
    let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake)?;
    server_stream.write_all(&server_handshake_bytes)?;

    //SERVER PEER ENVIA UN MENSAJE INVALIDO PARA CORTAR CONEXION ANTES DE TIEMPO
    let server_msg = P2PMessage::Bitfield {
        bitfield: vec![
            PieceStatus::ValidAndAvailablePiece,
            PieceStatus::ValidAndAvailablePiece,
            PieceStatus::ValidAndAvailablePiece,
        ],
    };
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream.write_all(&server_msg_bytes)?;

    //VERIFICACIONES
    let (local_peer, _torrent_status) = rx.recv()?;

    assert_eq!(
        P2PMessage::Handshake {
            protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            peer_id: local_peer.get_peer_id(),
        },
        received_handshake
    );
    let expected_id: Vec<u8> = DEFAULT_SERVER_PEER_ID.bytes().collect();
    assert_eq!(expected_id, local_peer.external_peer_data.peer_id);
    assert_eq!(
        vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
        local_peer.external_peer_data.pieces_availability
    );

    let _joined = handle.join();
    Ok(())
}

// #[test]
// fn client_peer_interact_with_a_peer_and_receives_one_block_ok() -> Result<(), Box<dyn Error>> {
//     //ABRO LA CONEXION
//     let (listener, address) = try_bind_listener(STARTING_PORT)?;
//     //CREACION DE UN CLIENTE PEER
//     let (tracker_response_data, mut torrent_status, torrent_file_data, mut local_peer) =
//         create_default_client_peer_with_a_server_peer_that_has_the_whole_file(
//             "test_one_block_ok.txt",
//             SocketAddr::from_str(&address)?,
//         )?;
//     let path = torrent_file_data.generate_torrent_representative_name();
//     fs::create_dir(format!("temp/{}", path))?;

//     //THREAD SECUNDARIO PARA EL CLIENTE
//     let (tx, rx) = mpsc::channel();
//     let handle = thread::spawn(move || {
//         //HANDLEO COMUNICACION
//         let result = msg_logic_control::interact_with_single_peer(
//             &mut local_peer,
//             0,
//             &torrent_file_data,
//             &tracker_response_data,
//             &mut torrent_status,
//         );
//         tx.send((local_peer, torrent_status, result))
//     });

//     let (mut server_stream, _addr) = listener.accept()?;

//     //SERVER PEER RECIBE UN HANDSHAKE
//     let received_message = msg_receiver::receive_handshake(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Handshake {
//             protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//             info_hash: DEFAULT_INFO_HASH.to_vec(),
//             peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN HANDSHAKE DE RESPUESTA
//     let server_handshake = P2PMessage::Handshake {
//         protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//         info_hash: DEFAULT_INFO_HASH.to_vec(),
//         peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
//     };
//     let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake)?;
//     server_stream.write_all(&server_handshake_bytes)?;

//     //SERVER PEER ENVIA UN BITFIELD
//     let server_msg = P2PMessage::Bitfield {
//         bitfield: vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//         ],
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIVE UN INTERESTED
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(P2PMessage::Interested, received_message);

//     //SERVER PEER ENVIA UN UNCHOKE
//     let server_msg = P2PMessage::Unchoke;
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIBE UN REQUEST
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: 0,
//             amount_of_bytes: BLOCK_BYTES.try_into()?
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN BLOQUE QUE CORRESPONDE A LA PIEZA ENTERA
//     let server_msg = P2PMessage::Piece {
//         piece_index: 0,
//         beginning_byte_index: 0,
//         block: [10; BLOCK_BYTES as usize].to_vec(),
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER ENVIA UN MENSAJE INVALIDO PARA CORTAR CONEXION ANTES DE TIEMPO
//     let server_msg = P2PMessage::Bitfield {
//         bitfield: vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//         ],
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //VEO QUE LE HAYA LLEGADO Y QUE ADEMAS LO ACEPTE
//     let (client_peer, torrent_status, result) = rx.recv()?;
//     assert_eq!(
//         Err(MsgLogicControlError::UpdatingBitfield(
//             "CheckingBitfield(\"[TorrentFileDataError] Some of the spare bits are set.\")"
//                 .to_string()
//         )),
//         result
//     );

//     let expected_id: Vec<u8> = DEFAULT_SERVER_PEER_ID.bytes().collect();
//     assert_eq!(expected_id, client_peer.external_peer_data.peer_id);
//     assert_eq!(true, client_peer.external_peer_data.am_choking);
//     assert_eq!(false, client_peer.external_peer_data.peer_choking);
//     assert_eq!(true, client_peer.external_peer_data.am_interested);

//     assert_eq!(
//         vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece
//         ],
//         client_peer.external_peer_data.pieces_availability
//     );

//     assert_eq!(0, torrent_status.uploaded);
//     assert_eq!(u64::from(BLOCK_BYTES), torrent_status.downloaded);
//     assert_eq!(
//         u64::try_from(DEFAULT_PIECE_LENGHT + DEFAULT_LAST_PIECE_LENGHT)? - u64::from(BLOCK_BYTES),
//         torrent_status.left
//     );
//     assert_eq!(
//         vec![
//             PieceStatus::PartiallyDownloaded {
//                 downloaded_bytes: BLOCK_BYTES
//             },
//             PieceStatus::MissingPiece
//         ],
//         torrent_status.pieces_availability
//     );

//     let _joined = handle.join();
//     let _result_of_removing = fs::remove_dir_all(format!("temp/{}", path));

//     Ok(())
// }

// #[test]
// fn client_peer_interact_with_a_peer_and_completes_a_piece_ok() -> Result<(), Box<dyn Error>> {
//     // ABRO LA CONEXION
//     let (listener, address) = try_bind_listener(STARTING_PORT)?;

//     // CREACION DE UN CLIENTE PEER
//     let (tracker_response_data, mut torrent_status, torrent_file_data, mut local_peer) =
//         create_default_client_peer_with_a_server_peer_that_has_the_whole_file(
//             "test_complete_piece_ok.txt",
//             SocketAddr::from_str(&address)?,
//         )?;
//     let path = torrent_file_data.generate_torrent_representative_name();
//     fs::create_dir(format!("temp/{}", path))?;

//     //THREAD SECUNDARIO PARA EL CLIENTE
//     let (tx, rx) = mpsc::channel();
//     let handle = thread::spawn(move || {
//         //HANDLEO COMUNICACION
//         let result = msg_logic_control::interact_with_single_peer(
//             &mut local_peer,
//             0,
//             &torrent_file_data,
//             &tracker_response_data,
//             &mut torrent_status,
//         );
//         tx.send((local_peer, torrent_status, result))
//     });

//     let (mut server_stream, _addr) = listener.accept()?;

//     //SERVER PEER RECIBE UN HANDSHAKE
//     let received_message = msg_receiver::receive_handshake(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Handshake {
//             protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//             info_hash: DEFAULT_INFO_HASH.to_vec(),
//             peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN HANDSHAKE DE RESPUESTA
//     let server_handshake = P2PMessage::Handshake {
//         protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//         info_hash: DEFAULT_INFO_HASH.to_vec(),
//         peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
//     };
//     let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake)?;
//     server_stream.write_all(&server_handshake_bytes)?;

//     //SERVER PEER ENVIA UN BITFIELD
//     let server_msg = P2PMessage::Bitfield {
//         bitfield: vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//         ],
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIVE UN INTERESTED
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(P2PMessage::Interested, received_message);

//     //SERVER PEER ENVIA UN UNCHOKE
//     let server_msg = P2PMessage::Unchoke;
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIBE UN REQUEST
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: 0,
//             amount_of_bytes: BLOCK_BYTES.try_into()?
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN BLOQUE
//     let server_msg = P2PMessage::Piece {
//         piece_index: 0,
//         beginning_byte_index: 0,
//         block: [10; BLOCK_BYTES as usize].to_vec(),
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIBE UN REQUEST
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: BLOCK_BYTES,
//             amount_of_bytes: BLOCK_BYTES.try_into()?
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN BLOQUE
//     let server_msg = P2PMessage::Piece {
//         piece_index: 0,
//         beginning_byte_index: BLOCK_BYTES,
//         block: [10; BLOCK_BYTES as usize].to_vec(),
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIBE UN REQUEST
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: BLOCK_BYTES * 2,
//             amount_of_bytes: (u32::try_from(DEFAULT_PIECE_LENGHT)?
//                 - 2 * u32::try_from(BLOCK_BYTES)?)
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN CHOKE
//     let server_msg = P2PMessage::Choke;
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIVE UN INTERESTED
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(P2PMessage::Interested, received_message);

//     //SERVER PEER ENVIA UN UNCHOKE
//     let server_msg = P2PMessage::Unchoke;
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER ENVIA UN BLOQUE
//     let server_msg = P2PMessage::Piece {
//         piece_index: 0,
//         beginning_byte_index: BLOCK_BYTES * 2,
//         block: [10; DEFAULT_PIECE_LENGHT - 2 * BLOCK_BYTES as usize].to_vec(),
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER ENVIA UN MENSAJE INVALIDO PARA CORTAR CONEXION ANTES DE TIEMPO
//     let server_msg = P2PMessage::Bitfield {
//         bitfield: vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//         ],
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //VERIFICACION
//     let (client_peer, torrent_status, result) = rx.recv()?;
//     assert_eq!(Ok(HandlerInteractionStatus::FinishInteraction), result);

//     let expected_id: Vec<u8> = DEFAULT_SERVER_PEER_ID.bytes().collect();
//     assert_eq!(expected_id, client_peer.external_peer_data.peer_id);
//     assert_eq!(true, client_peer.external_peer_data.am_choking);
//     assert_eq!(false, client_peer.external_peer_data.peer_choking);
//     assert_eq!(true, client_peer.external_peer_data.am_interested);

//     assert_eq!(
//         vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece
//         ],
//         client_peer.external_peer_data.pieces_availability
//     );

//     assert_eq!(0, torrent_status.uploaded);
//     assert_eq!(
//         u64::try_from(DEFAULT_PIECE_LENGHT)?,
//         torrent_status.downloaded
//     );
//     assert_eq!(
//         u64::try_from(DEFAULT_LAST_PIECE_LENGHT)?,
//         torrent_status.left
//     );
//     assert_eq!(
//         vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::MissingPiece
//         ],
//         torrent_status.pieces_availability
//     );

//     let _joined = handle.join();
//     let _result_of_removing = fs::remove_dir_all(format!("temp/{}", path));

//     Ok(())
// }

// #[test]
// fn client_peer_interact_with_a_peer_that_shuts_down_the_connection_and_then_interacts_with_another_one_correctly_ok(
// ) -> Result<(), Box<dyn Error>> {
//     // ABRO LA CONEXION
//     let (listener, address) = try_bind_listener(STARTING_PORT)?;
//     let (listener2, address2) = try_bind_listener(STARTING_PORT)?;

//     // CREACION DE CLIENTE PEER
//     let (tracker_response_data, mut torrent_status, torrent_file_data, mut local_peer) =
//         create_default_client_peer_with_a_server_peer_that_has_the_whole_file(
//             "test_complete_piece_with_shutdown_ok.txt",
//             SocketAddr::from_str(&address)?,
//         )?;
//     let (tracker_response_data2, mut torrent_status2, torrent_file_data2, mut local_peer2) =
//         create_default_client_peer_with_a_server_peer_that_has_the_whole_file(
//             "test_complete_piece_with_shutdown_ok.txt",
//             SocketAddr::from_str(&address2)?,
//         )?;

//     //THREAD SECUNDARIO PARA EL CLIENTE
//     let (tx, rx) = mpsc::channel();
//     let handle = thread::spawn(move || {
//         //HANDLEO COMUNICACION
//         let result = handler::handle_general_interaction_as_client(
//             &mut local_peer,
//             &torrent_file_data,
//             &tracker_response_data,
//             &mut torrent_status,
//         );
//         tx.send((local_peer, torrent_status, result))
//     });

//     // let (tx2, _rx2) = mpsc::channel();
//     // let handle2 = thread::spawn(move || {
//     //     //HANDLEO COMUNICACION
//     //     let _result2 = handler::handle_general_interaction_as_client(
//     //         &mut local_peer2,
//     //         &torrent_file_data2,
//     //         &tracker_response_data2,
//     //         &mut torrent_status2,
//     //     );
//     //     tx2.send((local_peer2, torrent_status2))
//     // });

//     // (SE MANTIENEN ABIERTAS LAS CONEXIONES DE LOS SERVER PEERS)
//     let (mut server_stream2, _addr2) = listener2.accept()?;
//     let (mut server_stream, _addr) = listener.accept()?;

//     //SERVER PEER 2 RECIBE UN HANDSHAKE
//     let received_message2 = msg_receiver::receive_handshake(&mut server_stream2)?;
//     assert_eq!(
//         P2PMessage::Handshake {
//             protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//             info_hash: DEFAULT_INFO_HASH.to_vec(),
//             peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
//         },
//         received_message2
//     );

//     //SERVER PEER 2 ENVIA UN HANDSHAKE DE RESPUESTA
//     let server_handshake2 = P2PMessage::Handshake {
//         protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//         info_hash: DEFAULT_INFO_HASH.to_vec(),
//         peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
//     };
//     let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake2)?;
//     server_stream2.write_all(&server_handshake_bytes)?;

//     //SERVER PEER 2 ENVIA UN BITFIELD
//     let server_msg2 = P2PMessage::Bitfield {
//         bitfield: vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//         ],
//     };
//     let server_msg_bytes2 = p2p::encoder::to_bytes(server_msg2)?;
//     server_stream2.write_all(&server_msg_bytes2)?;

//     //SERVER PEER 2 RECIVE UN INTERESTED
//     let received_message2 = msg_receiver::receive_message(&mut server_stream2)?;
//     assert_eq!(P2PMessage::Interested, received_message2);

//     //SERVER PEER 2 ENVIA UN UNCHOKE
//     let server_msg2 = P2PMessage::Unchoke;
//     let server_msg_bytes2 = p2p::encoder::to_bytes(server_msg2)?;
//     server_stream2.write_all(&server_msg_bytes2)?;

//     //SERVER PEER 2 RECIBE UN REQUEST
//     let received_message2 = msg_receiver::receive_message(&mut server_stream2)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: 0,
//             amount_of_bytes: BLOCK_BYTES.try_into()?
//         },
//         received_message2
//     );

//     //SERVER PEER 2 ENVIA UN BLOQUE
//     let server_msg2 = P2PMessage::Piece {
//         piece_index: 0,
//         beginning_byte_index: 0,
//         block: [10; BLOCK_BYTES as usize].to_vec(),
//     };
//     let server_msg_bytes2 = p2p::encoder::to_bytes(server_msg2)?;
//     server_stream2.write_all(&server_msg_bytes2)?;

//     //SERVER PEER 2 RECIBE UN REQUEST
//     let received_message2 = msg_receiver::receive_message(&mut server_stream2)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: BLOCK_BYTES,
//             amount_of_bytes: BLOCK_BYTES.try_into()?
//         },
//         received_message2
//     );

//     //SERVER PEER 2 CORTA LA CONEXION DE MANERA INESPERADA
//     server_stream2.shutdown(std::net::Shutdown::Both)?;

//     //SERVER PEER (1) RECIBE UN HANDSHAKE
//     let received_message = msg_receiver::receive_handshake(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Handshake {
//             protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//             info_hash: DEFAULT_INFO_HASH.to_vec(),
//             peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN HANDSHAKE DE RESPUESTA
//     let server_handshake = P2PMessage::Handshake {
//         protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
//         info_hash: DEFAULT_INFO_HASH.to_vec(),
//         peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
//     };
//     let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake)?;
//     server_stream.write_all(&server_handshake_bytes)?;

//     //SERVER PEER ENVIA UN BITFIELD
//     let server_msg = P2PMessage::Bitfield {
//         bitfield: vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//         ],
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIVE UN INTERESTED
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(P2PMessage::Interested, received_message);

//     //SERVER PEER ENVIA UN UNCHOKE
//     let server_msg = P2PMessage::Unchoke;
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIBE UN REQUEST
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: 0,
//             amount_of_bytes: BLOCK_BYTES.try_into()?
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN BLOQUE
//     let server_msg = P2PMessage::Piece {
//         piece_index: 0,
//         beginning_byte_index: 0,
//         block: [10; BLOCK_BYTES as usize].to_vec(),
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIBE UN REQUEST
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: BLOCK_BYTES,
//             amount_of_bytes: BLOCK_BYTES.try_into()?
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN BLOQUE
//     let server_msg = P2PMessage::Piece {
//         piece_index: 0,
//         beginning_byte_index: BLOCK_BYTES,
//         block: [10; BLOCK_BYTES as usize].to_vec(),
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIBE UN REQUEST
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: BLOCK_BYTES * 2,
//             amount_of_bytes: (u32::try_from(DEFAULT_PIECE_LENGHT)?
//                 - 2 * u32::try_from(BLOCK_BYTES)?)
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN CHOKE
//     let server_msg = P2PMessage::Choke;
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIBE UN INTERESTED
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(P2PMessage::Interested, received_message);

//     //SERVER PEER ENVIA UN UNCHOKE
//     let server_msg = P2PMessage::Unchoke;
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER RECIBE UN REQUEST
//     let received_message = msg_receiver::receive_message(&mut server_stream)?;
//     assert_eq!(
//         P2PMessage::Request {
//             piece_index: 0,
//             beginning_byte_index: BLOCK_BYTES * 2,
//             amount_of_bytes: (u32::try_from(DEFAULT_PIECE_LENGHT)?
//                 - 2 * u32::try_from(BLOCK_BYTES)?)
//         },
//         received_message
//     );

//     //SERVER PEER ENVIA UN BLOQUE
//     let server_msg = P2PMessage::Piece {
//         piece_index: 0,
//         beginning_byte_index: BLOCK_BYTES * 2,
//         block: [10; DEFAULT_PIECE_LENGHT - 2 * BLOCK_BYTES as usize].to_vec(),
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //SERVER PEER ENVIA UN MENSAJE INVALIDO PARA CORTAR CONEXION ANTES DE TIEMPO
//     let server_msg = P2PMessage::Bitfield {
//         bitfield: vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece,
//         ],
//     };
//     let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
//     server_stream.write_all(&server_msg_bytes)?;

//     //VERIFICACION
//     let (client_peer, torrent_status, result) = rx.recv()?;
//     assert_eq!(Ok(()), result);

//     let expected_id: Vec<u8> = DEFAULT_SERVER_PEER_ID.bytes().collect();
//     assert_eq!(expected_id, client_peer.external_peer_data.peer_id);
//     assert_eq!(true, client_peer.external_peer_data.am_choking);
//     assert_eq!(false, client_peer.external_peer_data.peer_choking);
//     assert_eq!(true, client_peer.external_peer_data.am_interested);

//     assert_eq!(
//         vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::ValidAndAvailablePiece
//         ],
//         client_peer.external_peer_data.pieces_availability
//     );

//     assert_eq!(0, torrent_status.uploaded);
//     assert_eq!(
//         u64::try_from(DEFAULT_PIECE_LENGHT)?,
//         torrent_status.downloaded
//     );
//     assert_eq!(
//         u64::try_from(DEFAULT_LAST_PIECE_LENGHT)?,
//         torrent_status.left
//     );
//     assert_eq!(
//         vec![
//             PieceStatus::ValidAndAvailablePiece,
//             PieceStatus::MissingPiece
//         ],
//         torrent_status.pieces_availability
//     );

//     let _joined = handle.join();
//     let _result_of_removing =
//         fs::remove_dir_all(format!("temp/test_complete_piece_with_shutdown_ok"));

//     Ok(())
// }
