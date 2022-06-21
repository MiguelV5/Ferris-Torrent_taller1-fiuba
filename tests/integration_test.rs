use core::fmt;
use fa_torrent::torrent::{
    client::peers_comunication::{
        handler::BLOCK_BYTES,
        //InteractionHandlerStatus
        msg_receiver,
    },
    data::{
        torrent_file_data::{TargetFilesData, TorrentFileData},
        torrent_status::{StateOfDownload, TorrentStatus},
        tracker_response_data::{PeerDataFromTrackerResponse, TrackerResponseData},
    },
    local_peer::{
        generate_peer_id, InteractionHandlerError, InteractionHandlerErrorKind, LocalPeer,
    },
    parsers::{
        p2p,
        p2p::constants::PSTR_STRING_HANDSHAKE,
        p2p::message::{P2PMessage, PieceStatus},
    },
    server::listener_binder::*,
};
use std::{
    error::Error,
    fs,
    io::Write,
    net::{SocketAddr, TcpListener},
    str::FromStr,
    sync::mpsc,
    thread,
};

pub const DEFAULT_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";
pub const DEFAULT_SERVER_PEER_ID2: &str = "-FA0001-000000000002";
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
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for TestingError {}

//=============================================================================================
// FUNCIONES AUXILIARES:
//

//=================================================
// RELACIONADAS A CREACION DE DATOS POR DEFECTO:

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

fn create_default_client_peer_with_two_server_peers_that_have_the_whole_file(
    torrent_name: &str,
    peer_address1: SocketAddr,
    peer_address2: SocketAddr,
) -> Result<(TrackerResponseData, TorrentStatus, TorrentFileData), Box<dyn Error>> {
    let server_peer = PeerDataFromTrackerResponse {
        peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
        peer_address: peer_address1.clone(),
    };
    let server_peer2 = PeerDataFromTrackerResponse {
        peer_id: Some(DEFAULT_SERVER_PEER_ID2.bytes().collect()),
        peer_address: peer_address2.clone(),
    };
    let tracker_response = TrackerResponseData {
        interval: 0,
        complete: 2,
        incomplete: 0,
        peers: vec![server_peer, server_peer2],
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
            35, 95, 100, 198, 139, 237, 56, 161, 225, 113, 168, 52, 228, 26, 36, 103, 150, 103, 76,
            233, 34,
        ],
        url_tracker_main: "tracker_main.com".to_string(),
        url_tracker_list: vec![],
        sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
        piece_length: 34000,
        total_amount_of_pieces: 2,
        total_length: 40000,
    };
    Ok((tracker_response, torrent_status, torrent_file))
}

fn create_default_client_peer_with_a_server_peer_that_has_the_whole_file(
    torrent_name: &str,
    peer_address: SocketAddr,
) -> Result<(TrackerResponseData, TorrentStatus, TorrentFileData), Box<dyn Error>> {
    let server_peer = PeerDataFromTrackerResponse {
        peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
        peer_address: peer_address.clone(),
    };
    let tracker_response = TrackerResponseData {
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
        // sha1_pieces: vec![
        //     46, 101, 88, 42, 242, 153, 87, 30, 42, 117, 240, 135, 191, 37, 12, 42, 175, 156, 136,
        //     214, 95, 100, 198, 139, 237, 56, 161, 225, 113, 168, 52, 228, 26, 36, 103, 150, 103,
        //     76, 233, 34,
        // ], MIGUEL: De donde sale este sha1 de la 1ra pieza? porque me estaba fallando a la hora de testear la pieza completa. El sha1 correspondiente que me dio es:
        //"ca9f2e3d336085c432baf198d63f83c678965323":
        sha1_pieces: vec![
            202, 159, 46, 61, 51, 96, 133, 196, 50, 186, 241, 152, 214, 63, 131, 198, 120, 150, 83,
            35, 95, 100, 198, 139, 237, 56, 161, 225, 113, 168, 52, 228, 26, 36, 103, 150, 103, 76,
            233, 34,
        ], //Ojo que a partir del 96 de la segunda linea (96 inclusive) es el sha1 de la 2da pieza. Creo que esa estaba bien de antes
        url_tracker_main: "tracker_main.com".to_string(),
        url_tracker_list: vec![],
        sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
        piece_length: 34000,
        total_amount_of_pieces: 2,
        total_length: 40000,
    };
    Ok((tracker_response, torrent_status, torrent_file))
}

//=================================================

// MOCKS DE PEERS SERVERS:

fn server_peer_interaction_mock_for_handshake(
    listener: TcpListener,
) -> Result<P2PMessage, Box<dyn Error>> {
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
    Ok(received_handshake)
}

fn server_peer_interaction_mock_for_receiving_one_block(
    listener: TcpListener,
) -> Result<(), Box<dyn Error>> {
    let (mut server_stream, _addr) = listener.accept()?;

    //SERVER PEER RECIBE UN HANDSHAKE
    let received_message = msg_receiver::receive_handshake(&mut server_stream)?;
    let expected_protocol_str_and_info_hash = (
        PSTR_STRING_HANDSHAKE.to_string(),
        DEFAULT_INFO_HASH.to_vec(),
    );
    if let P2PMessage::Handshake {
        protocol_str,
        info_hash,
        peer_id: _,
    } = received_message
    {
        assert_eq!(
            expected_protocol_str_and_info_hash,
            (protocol_str, info_hash)
        );
    };

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
        bitfield: vec![
            PieceStatus::ValidAndAvailablePiece,
            PieceStatus::ValidAndAvailablePiece,
        ],
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
            amount_of_bytes: BLOCK_BYTES.try_into()?
        },
        received_message
    );

    //SERVER PEER ENVIA UN BLOQUE QUE CORRESPONDE A LA PIEZA ENTERA
    let server_msg = P2PMessage::Piece {
        piece_index: 0,
        beginning_byte_index: 0,
        block: [10; BLOCK_BYTES as usize].to_vec(),
    };
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream.write_all(&server_msg_bytes)?;

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
    Ok(())
}

// fn server_peer_interaction_mock_for_receiving_an_entire_piece(
//     listener: TcpListener,
// ) -> Result<(), Box<dyn Error>> {
//     let (mut server_stream, _addr) = listener.accept()?;

//     //SERVER PEER RECIBE UN HANDSHAKE
//     let received_message = msg_receiver::receive_handshake(&mut server_stream)?;
//     let expected_protocol_str_and_info_hash = (
//         PSTR_STRING_HANDSHAKE.to_string(),
//         DEFAULT_INFO_HASH.to_vec(),
//     );
//     if let P2PMessage::Handshake {
//         protocol_str,
//         info_hash,
//         peer_id: _,
//     } = received_message
//     {
//         assert_eq!(
//             expected_protocol_str_and_info_hash,
//             (protocol_str, info_hash)
//         );
//     };

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
//     Ok(())
// }

fn server_peer_interaction_mock_for_receiving_an_entire_piece_with_shutdown(
    listener: TcpListener,
) -> Result<(), Box<dyn Error>> {
    // (SE MANTIENEN ABIERTAS LAS CONEXIONES DE LOS SERVER PEERS)
    let (mut server_stream_prone_to_shut, _addr_prone_to_shut) = listener.accept()?;

    //SERVER PEER propenso a hacer shutdown RECIBE UN HANDSHAKE
    let received_message = msg_receiver::receive_handshake(&mut server_stream_prone_to_shut)?;
    let expected_protocol_str_and_info_hash = (
        PSTR_STRING_HANDSHAKE.to_string(),
        DEFAULT_INFO_HASH.to_vec(),
    );
    if let P2PMessage::Handshake {
        protocol_str,
        info_hash,
        peer_id: _,
    } = received_message
    {
        assert_eq!(
            expected_protocol_str_and_info_hash,
            (protocol_str, info_hash)
        );
    };

    //SERVER PEER propenso a hacer shutdown ENVIA UN HANDSHAKE DE RESPUESTA
    let server_handshake = P2PMessage::Handshake {
        protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
        info_hash: DEFAULT_INFO_HASH.to_vec(),
        peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
    };
    let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake)?;
    server_stream_prone_to_shut.write_all(&server_handshake_bytes)?;

    //SERVER PEER propenso a hacer shutdown ENVIA UN BITFIELD
    let server_msg = P2PMessage::Bitfield {
        bitfield: vec![
            PieceStatus::ValidAndAvailablePiece,
            PieceStatus::ValidAndAvailablePiece,
        ],
    };
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream_prone_to_shut.write_all(&server_msg_bytes)?;

    //SERVER PEER propenso a hacer shutdown RECIVE UN INTERESTED
    let received_message = msg_receiver::receive_message(&mut server_stream_prone_to_shut)?;
    assert_eq!(P2PMessage::Interested, received_message);

    //SERVER PEER propenso a hacer shutdown ENVIA UN UNCHOKE
    let server_msg = P2PMessage::Unchoke;
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream_prone_to_shut.write_all(&server_msg_bytes)?;

    //SERVER PEER propenso a hacer shutdown RECIBE UN REQUEST
    let received_message = msg_receiver::receive_message(&mut server_stream_prone_to_shut)?;
    assert_eq!(
        P2PMessage::Request {
            piece_index: 0,
            beginning_byte_index: 0,
            amount_of_bytes: BLOCK_BYTES.try_into()?
        },
        received_message
    );

    //SERVER PEER propenso a hacer shutdown ENVIA UN BLOQUE
    let server_msg = P2PMessage::Piece {
        piece_index: 0,
        beginning_byte_index: 0,
        block: [10; BLOCK_BYTES as usize].to_vec(),
    };
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream_prone_to_shut.write_all(&server_msg_bytes)?;

    //SERVER PEER propenso a hacer shutdown RECIBE UN REQUEST
    let received_message = msg_receiver::receive_message(&mut server_stream_prone_to_shut)?;
    assert_eq!(
        P2PMessage::Request {
            piece_index: 0,
            beginning_byte_index: BLOCK_BYTES,
            amount_of_bytes: BLOCK_BYTES.try_into()?
        },
        received_message
    );

    //SERVER PEER propenso a hacer shutdown CORTA LA CONEXION DE MANERA INESPERADA
    server_stream_prone_to_shut.shutdown(std::net::Shutdown::Both)?;
    Ok(())
}

fn server_peer_interaction_mock_for_receiving_an_entire_piece_without_shutdown(
    listener: TcpListener,
) -> Result<(), Box<dyn Error>> {
    let (mut server_stream, _addr) = listener.accept()?;

    //SERVER PEER RECIBE UN HANDSHAKE
    let received_message = msg_receiver::receive_handshake(&mut server_stream)?;
    let expected_protocol_str_and_info_hash = (
        PSTR_STRING_HANDSHAKE.to_string(),
        DEFAULT_INFO_HASH.to_vec(),
    );
    if let P2PMessage::Handshake {
        protocol_str,
        info_hash,
        peer_id: _,
    } = received_message
    {
        assert_eq!(
            expected_protocol_str_and_info_hash,
            (protocol_str, info_hash)
        );
    };

    //SERVER PEER ENVIA UN HANDSHAKE DE RESPUESTA
    let server_handshake = P2PMessage::Handshake {
        protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
        info_hash: DEFAULT_INFO_HASH.to_vec(),
        peer_id: DEFAULT_SERVER_PEER_ID2.bytes().collect(),
    };
    let server_handshake_bytes = p2p::encoder::to_bytes(server_handshake)?;
    server_stream.write_all(&server_handshake_bytes)?;

    //SERVER PEER ENVIA UN BITFIELD
    let server_msg = P2PMessage::Bitfield {
        bitfield: vec![
            PieceStatus::ValidAndAvailablePiece,
            PieceStatus::ValidAndAvailablePiece,
        ],
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
            amount_of_bytes: BLOCK_BYTES.try_into()?
        },
        received_message
    );

    //SERVER PEER ENVIA UN BLOQUE
    let server_msg = P2PMessage::Piece {
        piece_index: 0,
        beginning_byte_index: 0,
        block: [10; BLOCK_BYTES as usize].to_vec(),
    };
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream.write_all(&server_msg_bytes)?;

    //SERVER PEER RECIBE UN REQUEST
    let received_message = msg_receiver::receive_message(&mut server_stream)?;
    assert_eq!(
        P2PMessage::Request {
            piece_index: 0,
            beginning_byte_index: BLOCK_BYTES,
            amount_of_bytes: BLOCK_BYTES.try_into()?
        },
        received_message
    );

    //SERVER PEER ENVIA UN BLOQUE
    let server_msg = P2PMessage::Piece {
        piece_index: 0,
        beginning_byte_index: BLOCK_BYTES,
        block: [10; BLOCK_BYTES as usize].to_vec(),
    };
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream.write_all(&server_msg_bytes)?;

    //SERVER PEER RECIBE UN REQUEST
    let received_message = msg_receiver::receive_message(&mut server_stream)?;
    assert_eq!(
        P2PMessage::Request {
            piece_index: 0,
            beginning_byte_index: BLOCK_BYTES * 2,
            amount_of_bytes: (u32::try_from(DEFAULT_PIECE_LENGHT)?
                - 2 * u32::try_from(BLOCK_BYTES)?)
        },
        received_message
    );

    //SERVER PEER ENVIA UN CHOKE
    let server_msg = P2PMessage::Choke;
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream.write_all(&server_msg_bytes)?;

    //SERVER PEER RECIBE UN INTERESTED
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
            beginning_byte_index: BLOCK_BYTES * 2,
            amount_of_bytes: (u32::try_from(DEFAULT_PIECE_LENGHT)?
                - 2 * u32::try_from(BLOCK_BYTES)?)
        },
        received_message
    );

    //SERVER PEER ENVIA UN BLOQUE
    let server_msg = P2PMessage::Piece {
        piece_index: 0,
        beginning_byte_index: BLOCK_BYTES * 2,
        block: [10; DEFAULT_PIECE_LENGHT - 2 * BLOCK_BYTES as usize].to_vec(),
    };
    let server_msg_bytes = p2p::encoder::to_bytes(server_msg)?;
    server_stream.write_all(&server_msg_bytes)?;

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
    Ok(())
}
//=================================================
//=============================================================================================

//
// TESTS:
//

#[test]
fn client_peer_receives_a_handshake_ok() -> Result<(), Box<dyn Error>> {
    // CREO EL LISTENER PARA EL SERVER
    let (listener, address) = try_bind_listener(STARTING_PORT)?;

    // CREO INFO NECESARIA PARA INICIAR COMUNICACION
    let (tracker_response_data, _torrent_status, torrent_file_data) =
        create_default_torrent_data("test_handshake_ok.txt", SocketAddr::from_str(&address)?)?;

    // Channel para obtener el handshake obtenido del lado del server:
    let (tx, rx) = mpsc::channel();

    //THREAD SECUNDARIO PARA EL SERVER
    let handle = thread::spawn(move || {
        let handshake_received_by_server_peer =
            server_peer_interaction_mock_for_handshake(listener);
        if let Ok(handshake) = handshake_received_by_server_peer {
            let _sent = tx.send(handshake);
        }
    });

    let peer_id = generate_peer_id();

    //INICIO LA COMUNICACION DEL LADO DEL CLIENTE
    let local_peer =
        LocalPeer::start_communication(&torrent_file_data, &tracker_response_data, 0, peer_id)?;
    // Obtengo el handshake que le habia llegado al server mock
    let handshake_received_by_server_peer = rx.recv()?;

    //VERIFICACIONES
    assert_eq!(
        P2PMessage::Handshake {
            protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            peer_id: local_peer.get_peer_id(),
        },
        handshake_received_by_server_peer
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

#[test]
fn client_peer_interact_with_a_peer_and_receives_one_block_ok() -> Result<(), Box<dyn Error>> {
    //ABRO LA CONEXION
    let (listener, address) = try_bind_listener(STARTING_PORT)?;

    // CREO INFO NECESARIA PARA INICIAR COMUNICACION
    let (tracker_response_data, mut torrent_status, torrent_file_data) =
        create_default_client_peer_with_a_server_peer_that_has_the_whole_file(
            "test_one_block_ok.txt",
            SocketAddr::from_str(&address)?,
        )?;
    let path = torrent_file_data.get_torrent_representative_name();
    let _dir_creation = fs::create_dir(format!("temp/{}", path));

    //THREAD SECUNDARIO PARA EL SERVER
    let handle = thread::spawn(move || {
        let _result = server_peer_interaction_mock_for_receiving_one_block(listener);
    });

    let peer_id = generate_peer_id();

    let mut client_peer =
        LocalPeer::start_communication(&torrent_file_data, &tracker_response_data, 0, peer_id)?;

    let interaction_result =
        client_peer.interact_with_peer(&torrent_file_data, &mut torrent_status);

    //VEO QUE LE HAYA LLEGADO Y QUE ADEMAS LO ACEPTE
    assert_eq!(
        Err(InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UpdatingBitfield(
            "\n    CheckingBitfield(\n    \"[TorrentFileDataError] Some of the spare bits are set.\",\n)\n"
                .to_string()
        ))),
        interaction_result
    );

    let expected_id: Vec<u8> = DEFAULT_SERVER_PEER_ID.bytes().collect();
    assert_eq!(expected_id, client_peer.external_peer_data.peer_id);
    assert_eq!(true, client_peer.external_peer_data.am_choking);
    assert_eq!(false, client_peer.external_peer_data.peer_choking);
    assert_eq!(true, client_peer.external_peer_data.am_interested);

    assert_eq!(
        vec![
            PieceStatus::ValidAndAvailablePiece,
            PieceStatus::ValidAndAvailablePiece
        ],
        client_peer.external_peer_data.pieces_availability
    );

    assert_eq!(0, torrent_status.uploaded);
    assert_eq!(u64::from(BLOCK_BYTES), torrent_status.downloaded);
    assert_eq!(
        u64::try_from(DEFAULT_PIECE_LENGHT + DEFAULT_LAST_PIECE_LENGHT)? - u64::from(BLOCK_BYTES),
        torrent_status.left
    );
    assert_eq!(
        vec![
            PieceStatus::PartiallyDownloaded {
                downloaded_bytes: BLOCK_BYTES
            },
            PieceStatus::MissingPiece
        ],
        torrent_status.pieces_availability
    );

    let _joined = handle.join();
    let _result_of_removing = fs::remove_dir_all(format!("temp/{}", path));

    Ok(())
}

// #[test]
// fn client_peer_interact_with_a_peer_and_completes_a_piece_ok() -> Result<(), Box<dyn Error>> {
//     //ABRO LA CONEXION
//     let (listener, address) = try_bind_listener(STARTING_PORT)?;

//     // CREO INFO NECESARIA PARA INICIAR COMUNICACION
//     let (tracker_response_data, mut torrent_status, torrent_file_data) =
//         create_default_client_peer_with_a_server_peer_that_has_the_whole_file(
//             "test_complete_piece_ok.txt",
//             SocketAddr::from_str(&address)?,
//         )?;
//     let path = torrent_file_data.get_torrent_representative_name();
//     let _dir_creation = fs::create_dir(format!("temp/{}", path));

//     //THREAD SECUNDARIO PARA EL SERVER
//     let handle = thread::spawn(move || {
//         let _result = server_peer_interaction_mock_for_receiving_an_entire_piece(listener);
//     });

//     let peer_id = generate_peer_id();

//     let mut client_peer =
//         LocalPeer::start_communication(&torrent_file_data, &tracker_response_data, 0, peer_id)?;

//     let interaction_result =
//         client_peer.interact_with_peer(&torrent_file_data, &mut torrent_status);

//     //VERIFICACION
//     assert_eq!(
//         Ok(InteractionHandlerStatus::FinishInteraction),
//         interaction_result
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

#[test]
fn client_peer_interact_with_a_peer_that_shuts_down_the_connection_and_then_interacts_with_another_one_correctly_ok(
) -> Result<(), Box<dyn Error>> {
    //ABRO LA CONEXION
    let (listener, address) = try_bind_listener(STARTING_PORT)?;
    let (listener2, address2) = try_bind_listener(STARTING_PORT)?;

    // CREO INFO NECESARIA PARA INICIAR COMUNICACION
    let (tracker_response_data, mut torrent_status, torrent_file_data) =
        create_default_client_peer_with_two_server_peers_that_have_the_whole_file(
            "test_complete_piece_with_shutdown_ok.txt",
            SocketAddr::from_str(&address)?,
            SocketAddr::from_str(&address2)?,
        )?; //estan en este orden a proposito. no cambiar

    let path = torrent_file_data.get_torrent_representative_name();
    let _dir_creation = fs::create_dir(format!("temp/{}", path));

    //THREADS SECUNDARIOS PARA LOS SERVERS
    let handle = thread::spawn(move || {
        let _result =
            server_peer_interaction_mock_for_receiving_an_entire_piece_with_shutdown(listener);
    });
    let handle2 = thread::spawn(move || {
        let _result =
            server_peer_interaction_mock_for_receiving_an_entire_piece_without_shutdown(listener2);
    });
    let peer_id = generate_peer_id();

    //INTERACTUA CLIENTE QUE CORTA CONEXION:
    let mut client_peer = LocalPeer::start_communication(
        &torrent_file_data,
        &tracker_response_data,
        0,
        peer_id.clone(),
    )?;
    let interaction_result =
        client_peer.interact_with_peer(&torrent_file_data, &mut torrent_status);
    assert!(interaction_result.is_err());

    //FLUSH:
    let torrent_path = torrent_file_data.get_torrent_representative_name();
    let _result = fs::remove_dir_all(format!("temp/{}", torrent_path))
        .map_err(|err| InteractionHandlerError::RestartingDownload(format!("{}", err)));
    fs::create_dir(format!("temp/{}", torrent_path))
        .map_err(|err| InteractionHandlerError::RestartingDownload(format!("{}", err)))?;

    torrent_status.flush_data(torrent_file_data.total_length as u64);

    //INTERACTUA CLIENTE QUE DEBERIA FINALIZAR LA PIEZA CORRECTAMENTE:
    let mut client_peer =
        LocalPeer::start_communication(&torrent_file_data, &tracker_response_data, 1, peer_id)?;
    let interaction_result =
        client_peer.interact_with_peer(&torrent_file_data, &mut torrent_status);
    assert!(interaction_result.is_ok());

    //VERIFICACION

    let expected_id: Vec<u8> = DEFAULT_SERVER_PEER_ID2.bytes().collect();
    assert_eq!(expected_id, client_peer.external_peer_data.peer_id);
    assert_eq!(true, client_peer.external_peer_data.am_choking);
    assert_eq!(false, client_peer.external_peer_data.peer_choking);
    assert_eq!(true, client_peer.external_peer_data.am_interested);

    assert_eq!(
        vec![
            PieceStatus::ValidAndAvailablePiece,
            PieceStatus::ValidAndAvailablePiece
        ],
        client_peer.external_peer_data.pieces_availability
    );

    assert_eq!(0, torrent_status.uploaded);
    assert_eq!(
        u64::try_from(DEFAULT_PIECE_LENGHT)?,
        torrent_status.downloaded
    );
    assert_eq!(
        u64::try_from(DEFAULT_LAST_PIECE_LENGHT)?,
        torrent_status.left
    );
    assert_eq!(
        vec![
            PieceStatus::ValidAndAvailablePiece,
            PieceStatus::MissingPiece
        ],
        torrent_status.pieces_availability
    );

    let _joined = handle2.join();
    let _joined = handle.join();
    let _result_of_removing =
        fs::remove_dir_all(format!("temp/test_complete_piece_with_shutdown_ok"));

    Ok(())
}
