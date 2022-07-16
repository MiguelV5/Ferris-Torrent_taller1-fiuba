//! # Modulo de envio de mensajes P2P
//! Este modulo contiene las funciones encargadas de enviar mensajes P2P en sockets, los cuales deben enviarse en bytes correspondientes al protocolo BitTorrent para comunicación entre peers
//!

use log::info;

use crate::torrent::data::{torrent_file_data::TorrentFileData, torrent_status::TorrentStatus};

use shared::parsers::{p2p, p2p::constants::PSTR_STRING_HANDSHAKE, p2p::message::P2PMessage};

use core::fmt;
use std::{error::Error, io::Write, net::TcpStream};

#[derive(PartialEq, Debug, Clone)]
pub enum MsgSenderError {
    EncondingMessageIntoBytes(String),
    WriteToTcpStream(String),
    ZeroAmountOfBytes(String),
    AmountOfBytesLimitExceeded(String),
    ZeroBlockLength(String),
    BlockLengthLimitExceeded(String),
    NumberConversion(String),
    SendingRequest(String),
}

impl fmt::Display for MsgSenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for MsgSenderError {}

const MAX_BLOCK_BYTES: u32 = 131072; //2^17 bytes

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Handshake
///
pub fn send_handshake(
    stream: &mut TcpStream,
    peer_id: &[u8],
    torrent_file_data: &TorrentFileData,
) -> Result<(), MsgSenderError> {
    let handshake_bytes = p2p::encoder::to_bytes(P2PMessage::Handshake {
        protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
        info_hash: torrent_file_data.get_info_hash(),
        peer_id: peer_id.to_vec(),
    })
    .map_err(|error| MsgSenderError::EncondingMessageIntoBytes(format!("{}", error)))?;

    stream
        .write_all(&handshake_bytes)
        .map_err(|error| MsgSenderError::WriteToTcpStream(format!("{}", error)))?;
    Ok(())
}

fn send_msg(stream: &mut TcpStream, msg_variant: P2PMessage) -> Result<(), MsgSenderError> {
    let msg_bytes = p2p::encoder::to_bytes(msg_variant)
        .map_err(|error| MsgSenderError::EncondingMessageIntoBytes(format!("{}", error)))?;

    stream
        .write_all(&msg_bytes)
        .map_err(|error| MsgSenderError::WriteToTcpStream(format!("{}", error)))?;
    Ok(())
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Keep Alive
///
pub fn send_keep_alive(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::KeepAlive)
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Choke
///
pub fn send_choke(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::Choke)
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Unchoke
///
pub fn send_unchoke(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::Unchoke)
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Interested
///
pub fn send_interested(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::Interested)
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Not Interested
///
pub fn send_not_interested(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::NotInterested)
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Have
///
pub fn send_have(stream: &mut TcpStream, completed_piece_index: u32) -> Result<(), MsgSenderError> {
    let have_msg = P2PMessage::Have {
        piece_index: completed_piece_index,
    };
    send_msg(stream, have_msg)
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Bitfield
///
pub fn send_bitfield(
    stream: &mut TcpStream,
    torrent_status: &TorrentStatus,
) -> Result<(), MsgSenderError> {
    let bitfield_msg = P2PMessage::Bitfield {
        bitfield: torrent_status.get_pieces_availability(),
    };
    send_msg(stream, bitfield_msg)
}

fn check_request_or_cancel_fields(amount_of_bytes: u32) -> Result<(), MsgSenderError> {
    if amount_of_bytes == 0 {
        return Err(MsgSenderError::ZeroAmountOfBytes(
            "[MsgSenderError] The amount of bytes cannot be equal zero.".to_string(),
        ));
    }

    if amount_of_bytes > MAX_BLOCK_BYTES {
        return Err(MsgSenderError::AmountOfBytesLimitExceeded(
            "[MsgSenderError] The amount of bytes must be smaller than 2^17.".to_string(),
        ));
    }
    Ok(())
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Request
///
pub fn send_request(
    stream: &mut TcpStream,
    torrent_file_data: &TorrentFileData,
    torrent_status: &TorrentStatus,
    piece_index: usize,
) -> Result<(), MsgSenderError> {
    let beginning_byte_index = torrent_status
        .calculate_beginning_byte_index(piece_index)
        .map_err(|err| MsgSenderError::SendingRequest(format!("{}", err)))?;
    let amount_of_bytes = torrent_status
        .calculate_amount_of_bytes_of_block(torrent_file_data, piece_index, beginning_byte_index)
        .map_err(|err| MsgSenderError::SendingRequest(format!("{}", err)))?;
    let piece_index = piece_index
        .try_into()
        .map_err(|error| MsgSenderError::SendingRequest(format!("{}", error)))?;

    //habria que ver si ese checkeo sigue siendo necesario
    check_request_or_cancel_fields(amount_of_bytes)?;
    let request_msg = P2PMessage::Request {
        piece_index,
        beginning_byte_index,
        amount_of_bytes,
    };
    send_msg(stream, request_msg)?;
    info!(
        "Mensaje enviado: Request[piece_index: {}, beginning_byte_index: {}. amount_of_bytes: {}]",
        piece_index, beginning_byte_index, amount_of_bytes
    );
    Ok(())
}

fn check_piece_fields(block: &[u8]) -> Result<(), MsgSenderError> {
    if block.is_empty() {
        return Err(MsgSenderError::ZeroBlockLength(
            "[MsgSenderError] The block length cannot be equal zero.".to_string(),
        ));
    }

    if block.len()
        > MAX_BLOCK_BYTES
            .try_into()
            .map_err(|error| MsgSenderError::NumberConversion(format!("{}", error)))?
    {
        return Err(MsgSenderError::BlockLengthLimitExceeded(
            "[MsgSenderError] The block length must be smaller than 2^17.".to_string(),
        ));
    }
    Ok(())
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Piece
///
pub fn send_piece(
    stream: &mut TcpStream,
    piece_index: u32,
    beginning_byte_index: u32,
    block: Vec<u8>,
) -> Result<(), MsgSenderError> {
    check_piece_fields(&block)?;
    let piece_msg = P2PMessage::Piece {
        piece_index,
        beginning_byte_index,
        block,
    };
    send_msg(stream, piece_msg)
}

/// Funcion encargada de codificar y enviar un mensaje P2P de tipo Cancel
///
pub fn send_cancel(
    stream: &mut TcpStream,
    torrent_file_data: &TorrentFileData,
    torrent_status: &TorrentStatus,
    piece_index: usize,
) -> Result<(), MsgSenderError> {
    let beginning_byte_index = torrent_status
        .calculate_beginning_byte_index(piece_index)
        .map_err(|err| MsgSenderError::SendingRequest(format!("{}", err)))?;
    let amount_of_bytes = torrent_status
        .calculate_amount_of_bytes_of_block(torrent_file_data, piece_index, beginning_byte_index)
        .map_err(|err| MsgSenderError::SendingRequest(format!("{}", err)))?;
    let piece_index = piece_index
        .try_into()
        .map_err(|error| MsgSenderError::SendingRequest(format!("{}", error)))?;

    //habria que ver si ese checkeo sigue siendo necesario
    check_request_or_cancel_fields(amount_of_bytes)?;
    let cancel_msg = P2PMessage::Cancel {
        piece_index,
        beginning_byte_index,
        amount_of_bytes,
    };
    send_msg(stream, cancel_msg)
}

#[cfg(test)]
mod test_msg_sender {
    use gtk::glib;

    use super::*;
    use crate::torrent::{
        client::peers_communication::{
            local_peer_communicator::{LocalPeerCommunicator, PeerRole},
            msg_receiver,
        },
        data::{
            peer_data_for_communication::PeerDataForP2PCommunication,
            torrent_file_data::{TargetFilesData, TorrentFileData},
            torrent_status::{StateOfDownload, TorrentStatus},
            tracker_response_data::{PeerDataFromTrackerResponse, TrackerResponseData},
        },
        port_testing::listener_binder::*,
    };

    use shared::parsers::p2p::{
        self,
        constants::{PSTR_STRING_HANDSHAKE, TOTAL_NUM_OF_BYTES_HANDSHAKE},
        message::{P2PMessage, PieceStatus},
    };

    use std::{
        error::Error, io::Read, net::SocketAddr, str::FromStr, sync::mpsc, time::SystemTime,
    };

    pub const DEFAULT_ADDR: &str = "127.0.0.1:8080";
    pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
    pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";
    pub const DEFAULT_INFO_HASH: [u8; 20] = [0; 20];

    ///==========================================
    //FUNCIONES AUXILIARES:

    fn create_default_client_peer_with_a_server_peer_that_has_one_piece() -> Result<
        (
            TrackerResponseData,
            TorrentStatus,
            TorrentFileData,
            LocalPeerCommunicator,
        ),
        Box<dyn Error>,
    > {
        let (_listener, address) = try_bind_listener(STARTING_PORT)?;
        let stream = TcpStream::connect(address)?;

        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
            peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
        };
        let tracker_response = TrackerResponseData {
            interval: 0,
            //tracker_id: DEFAULT_TRACKER_ID.to_string(),
            complete: 1,
            incomplete: 0,
            peers: vec![server_peer],
        };
        let torrent_status = TorrentStatus {
            uploaded: 0,
            downloaded: 0,
            left: 16,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::MissingPiece {
                was_requested: false,
            }],
        };
        let torrent_file = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: "nombre.txt".to_string(),
                file_length: 16,
            },
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
            sha1_pieces: vec![],
            piece_length: 16,
            total_amount_of_pieces: 1,
            total_length: 16,
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            pieces_availability: vec![PieceStatus::ValidAndAvailablePiece],
            am_interested: false,
            am_choking: true,
            peer_choking: true,
            peer_interested: false,
        };
        let (ui_sender, _) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        let local_peer = LocalPeerCommunicator {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            stream,
            external_peer_data: server_peer_data,
            role: PeerRole::Client,
            logger_sender: mpsc::channel().0,
            ui_sender: ui_sender,
            clock: SystemTime::now(),
        };
        Ok((tracker_response, torrent_status, torrent_file, local_peer))
    }

    //==========================================

    #[test]
    fn client_peer_send_a_handshake_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        let (_tracker_response, _torrent_status, torrent_file_data, local_peer) =
            create_default_client_peer_with_a_server_peer_that_has_one_piece()?;

        assert!(
            send_handshake(&mut sender_stream, &local_peer.peer_id, &torrent_file_data).is_ok()
        );

        let mut buffer = [0; TOTAL_NUM_OF_BYTES_HANDSHAKE];
        receptor_stream.read(&mut buffer)?;
        let received_msg = p2p::decoder::from_bytes(&buffer)?;

        let expected_msg = P2PMessage::Handshake {
            protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
            info_hash: torrent_file_data.sha1_info_hash.clone(),
            peer_id: local_peer.peer_id.clone(),
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_keep_alive_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        assert!(send_keep_alive(&mut sender_stream).is_ok());

        let mut buffer = [0; 4];
        receptor_stream.read(&mut buffer)?;
        let received_msg = p2p::decoder::from_bytes(&buffer)?;

        let expected_msg = P2PMessage::KeepAlive;

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_choke_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        assert!(send_choke(&mut sender_stream).is_ok());

        let mut buffer = [0; 5];
        receptor_stream.read(&mut buffer)?;
        let received_msg = p2p::decoder::from_bytes(&buffer)?;

        let expected_msg = P2PMessage::Choke;

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_unchoke_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        assert!(send_unchoke(&mut sender_stream).is_ok());

        let mut buffer = [0; 5];
        receptor_stream.read(&mut buffer)?;
        let received_msg = p2p::decoder::from_bytes(&buffer)?;

        let expected_msg = P2PMessage::Unchoke;

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_interested_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        assert!(send_interested(&mut sender_stream).is_ok());

        let mut buffer = [0; 5];
        receptor_stream.read(&mut buffer)?;
        let received_msg = p2p::decoder::from_bytes(&buffer)?;

        let expected_msg = P2PMessage::Interested;

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_not_interested_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        assert!(send_not_interested(&mut sender_stream).is_ok());

        let mut buffer = [0; 5];
        receptor_stream.read(&mut buffer)?;
        let received_msg = p2p::decoder::from_bytes(&buffer)?;

        let expected_msg = P2PMessage::NotInterested;

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_have_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        assert!(send_have(&mut sender_stream, 2).is_ok());

        let received_msg = msg_receiver::receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Have { piece_index: 2 };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_with_no_pieces_send_bitfield_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        let (_tracker_response, torrent_status, _torrent_file_data, _local_peer) =
            create_default_client_peer_with_a_server_peer_that_has_one_piece()?;

        assert!(send_bitfield(&mut sender_stream, &torrent_status).is_ok());

        let received_msg = msg_receiver::receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Bitfield {
            bitfield: vec![
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
            ],
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_with_some_pieces_send_bitfield_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        let (_tracker_response, mut torrent_status, _torrent_file_data, _local_peer) =
            create_default_client_peer_with_a_server_peer_that_has_one_piece()?;
        torrent_status.pieces_availability[0] = PieceStatus::ValidAndAvailablePiece;

        assert!(send_bitfield(&mut sender_stream, &torrent_status).is_ok());

        let received_msg = msg_receiver::receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Bitfield {
            bitfield: vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
                PieceStatus::MissingPiece {
                    was_requested: false,
                },
            ],
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_request_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;
        let torrent_file_data = TorrentFileData {
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            sha1_pieces: vec![],
            sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 8,
            total_length: 8,
            total_amount_of_pieces: 1,
            target_files_data: TargetFilesData::SingleFile {
                file_name: "name.txt".to_string(),
                file_length: 8,
            },
        };
        let torrent_status = TorrentStatus {
            uploaded: 0,
            downloaded: 4,
            left: 4,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::PartiallyDownloaded {
                downloaded_bytes: 4,
                was_requested: false,
            }],
        };

        assert!(send_request(&mut sender_stream, &torrent_file_data, &torrent_status, 0).is_ok());

        let received_msg = msg_receiver::receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Request {
            piece_index: 0,
            beginning_byte_index: 4,
            amount_of_bytes: 4,
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_piece_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        let block = vec![0, 1, 2, 3];
        assert!(send_piece(&mut sender_stream, 0, 4, block.clone()).is_ok());

        let received_msg = msg_receiver::receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Piece {
            piece_index: 0,
            beginning_byte_index: 4,
            block,
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_piece_of_zero_bytes_error() -> Result<(), Box<dyn Error>> {
        let (_listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;

        assert_eq!(
            Err(MsgSenderError::ZeroBlockLength(
                "[MsgSenderError] The block length cannot be equal zero.".to_string(),
            )),
            send_piece(&mut sender_stream, 0, 4, vec![])
        );

        Ok(())
    }

    #[test]
    fn client_peer_send_piece_bigger_than_the_max_block_bytes_error() -> Result<(), Box<dyn Error>>
    {
        let (_listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;

        assert_eq!(
            Err(MsgSenderError::BlockLengthLimitExceeded(
                "[MsgSenderError] The block length must be smaller than 2^17.".to_string(),
            )),
            send_piece(&mut sender_stream, 0, 4, [0; 131072 + 1].to_vec())
        );

        Ok(())
    }

    #[test]
    fn client_peer_send_cancel_ok() -> Result<(), Box<dyn Error>> {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;
        let mut sender_stream = TcpStream::connect(address)?;
        let (mut receptor_stream, _addr) = listener.accept()?;
        let torrent_file_data = TorrentFileData {
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            sha1_pieces: vec![],
            sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 8,
            total_length: 8,
            total_amount_of_pieces: 1,
            target_files_data: TargetFilesData::SingleFile {
                file_name: "name.txt".to_string(),
                file_length: 8,
            },
        };
        let torrent_status = TorrentStatus {
            uploaded: 0,
            downloaded: 4,
            left: 4,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::PartiallyDownloaded {
                downloaded_bytes: 4,
                was_requested: false,
            }],
        };

        assert!(send_cancel(&mut sender_stream, &torrent_file_data, &torrent_status, 0).is_ok());

        let received_msg = msg_receiver::receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Cancel {
            piece_index: 0,
            beginning_byte_index: 4,
            amount_of_bytes: 4,
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }
}
