#![allow(dead_code)]
use crate::torrent::client::client_struct::*;
use crate::torrent::parsers::{
    p2p, p2p::constants::PSTR_STRING_HANDSHAKE, p2p::message::P2PMessage,
};
use core::fmt;
use std::error::Error;
use std::io::Write;
use std::net::TcpStream;

#[derive(PartialEq, Debug, Clone)]
pub enum MsgSenderError {
    EncondingMessageIntoBytes(String),
    WriteToTcpStream(String),
    ZeroAmountOfBytes(String),
    AmountOfBytesLimitExceeded(String),
    ZeroBlockLength(String),
    BlockLengthLimitExceeded(String),
    NumberConversion(String),
}

impl fmt::Display for MsgSenderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for MsgSenderError {}

/* FALTA:
 * - Documentar
 */

const MAX_BLOCK_BYTES: u32 = 131072; //2^17 bytes

pub fn send_handshake(client_peer: &Client, stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    let handshake_bytes = p2p::encoder::to_bytes(P2PMessage::Handshake {
        protocol_str: PSTR_STRING_HANDSHAKE.to_string(), //esto capaz deberia ser un campo del cliente
        info_hash: client_peer.info_hash.clone(),
        peer_id: client_peer.peer_id.clone(),
    })
    .map_err(|error| MsgSenderError::EncondingMessageIntoBytes(format!("{:?}", error)))?;

    stream
        .write_all(&handshake_bytes)
        .map_err(|error| MsgSenderError::WriteToTcpStream(format!("{:?}", error)))?;
    Ok(())
}

fn send_msg(stream: &mut TcpStream, msg_variant: P2PMessage) -> Result<(), MsgSenderError> {
    let msg_bytes = p2p::encoder::to_bytes(msg_variant)
        .map_err(|error| MsgSenderError::EncondingMessageIntoBytes(format!("{:?}", error)))?;

    stream
        .write_all(&msg_bytes)
        .map_err(|error| MsgSenderError::WriteToTcpStream(format!("{:?}", error)))?;
    Ok(())
}

pub fn send_keep_alive(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::KeepAlive)
}

pub fn send_choke(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::Choke)
}

pub fn send_unchoke(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::Unchoke)
}

pub fn send_interested(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::Interested)
}

pub fn send_not_interested(stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    send_msg(stream, P2PMessage::NotInterested)
}

pub fn send_have(stream: &mut TcpStream, completed_piece_index: u32) -> Result<(), MsgSenderError> {
    let have_msg = P2PMessage::Have {
        piece_index: completed_piece_index,
    };
    send_msg(stream, have_msg)
}

pub fn send_bitfield(client_peer: &Client, stream: &mut TcpStream) -> Result<(), MsgSenderError> {
    let bitfield_msg = P2PMessage::Bitfield {
        bitfield: client_peer.data_of_download.pieces_availability.clone(),
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

pub fn send_request(
    stream: &mut TcpStream,
    piece_index: u32,
    beginning_byte_index: u32,
    amount_of_bytes: u32,
) -> Result<(), MsgSenderError> {
    check_request_or_cancel_fields(amount_of_bytes)?;
    let request_msg = P2PMessage::Request {
        piece_index,
        beginning_byte_index,
        amount_of_bytes,
    };
    send_msg(stream, request_msg)
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
            .map_err(|error| MsgSenderError::NumberConversion(format!("{:?}", error)))?
    {
        return Err(MsgSenderError::BlockLengthLimitExceeded(
            "[MsgSenderError] The block length must be smaller than 2^17.".to_string(),
        ));
    }
    Ok(())
}

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

pub fn send_cancel(
    stream: &mut TcpStream,
    piece_index: u32,
    beginning_byte_index: u32,
    amount_of_bytes: u32,
) -> Result<(), MsgSenderError> {
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
    use super::*;
    use crate::torrent::client::peers_comunication::msg_receiver::receive_message;
    use crate::torrent::data::data_of_download::{DataOfDownload, StateOfDownload};
    use crate::torrent::data::torrent_file_data::TorrentFileData;
    use crate::torrent::data::tracker_response_data::{
        PeerDataFromTrackerResponse, TrackerResponseData,
    };
    use crate::torrent::parsers::p2p::constants::PSTR_STRING_HANDSHAKE;
    use crate::torrent::parsers::p2p::constants::TOTAL_NUM_OF_BYTES_HANDSHAKE;
    use crate::torrent::parsers::{p2p, p2p::message::P2PMessage};
    use crate::torrent::{
        client::peers_comunication::msg_logic_control::{
            DEFAULT_ADDR, DEFAULT_CLIENT_PEER_ID, DEFAULT_INFO_HASH, DEFAULT_SERVER_PEER_ID,
            DEFAULT_TRACKER_ID,
        },
        data::peer_data_for_communication::PeerDataForP2PCommunication,
        parsers::p2p::message::PieceStatus,
    };
    use std::collections::HashMap;
    use std::error::Error;
    use std::io::Read;
    use std::net::{SocketAddr, TcpListener};
    use std::str::FromStr;
    use std::vec;

    fn create_default_client_peer_with_a_server_peer_that_has_one_piece(
    ) -> Result<Client, Box<dyn Error>> {
        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.to_string()),
            peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
        };
        let tracker_response = TrackerResponseData {
            interval: 0,
            tracker_id: DEFAULT_TRACKER_ID.to_string(),
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
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            info: HashMap::new(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 16,
            total_amount_pieces: 1,
            total_size: 16,
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.to_string(),
            pieces_availability: Some(vec![PieceStatus::ValidAndAvailablePiece]),
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

    #[test]
    fn client_peer_send_a_handshake_ok() -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        let client_peer = create_default_client_peer_with_a_server_peer_that_has_one_piece()?;

        assert!(send_handshake(&client_peer, &mut sender_stream).is_ok());

        let mut buffer = [0; TOTAL_NUM_OF_BYTES_HANDSHAKE];
        receptor_stream.read(&mut buffer)?;
        let received_msg = p2p::decoder::from_bytes(&buffer)?;

        let expected_msg = P2PMessage::Handshake {
            protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
            info_hash: client_peer.info_hash.clone(),
            peer_id: client_peer.peer_id.clone(),
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_keep_alive_ok() -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
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
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
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
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
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
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
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
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
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
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        assert!(send_have(&mut sender_stream, 2).is_ok());

        let received_msg = receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Have { piece_index: 2 };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_with_no_pieces_send_bitfield_ok() -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        let client_peer = create_default_client_peer_with_a_server_peer_that_has_one_piece()?;

        assert!(send_bitfield(&client_peer, &mut sender_stream).is_ok());

        let received_msg = receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Bitfield {
            bitfield: vec![
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
            ],
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_with_some_pieces_send_bitfield_ok() -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        let mut client_peer = create_default_client_peer_with_a_server_peer_that_has_one_piece()?;
        client_peer.data_of_download.pieces_availability[0] = PieceStatus::ValidAndAvailablePiece;

        assert!(send_bitfield(&client_peer, &mut sender_stream).is_ok());

        let received_msg = receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Bitfield {
            bitfield: vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
            ],
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_request_ok() -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        assert!(send_request(&mut sender_stream, 0, 4, 4).is_ok());

        let received_msg = receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Request {
            piece_index: 0,
            beginning_byte_index: 4,
            amount_of_bytes: 4,
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_request_of_zero_bytes_error() -> Result<(), Box<dyn Error>> {
        let _listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;

        assert_eq!(
            Err(MsgSenderError::ZeroAmountOfBytes(
                "[MsgSenderError] The amount of bytes cannot be equal zero.".to_string(),
            )),
            send_request(&mut sender_stream, 0, 4, 0)
        );

        Ok(())
    }

    #[test]
    fn client_peer_send_request_bigger_than_the_max_block_bytes_error() -> Result<(), Box<dyn Error>>
    {
        let _listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;

        assert_eq!(
            Err(MsgSenderError::AmountOfBytesLimitExceeded(
                "[MsgSenderError] The amount of bytes must be smaller than 2^17.".to_string(),
            )),
            send_request(&mut sender_stream, 0, 4, MAX_BLOCK_BYTES + 1)
        );

        Ok(())
    }

    #[test]
    fn client_peer_send_piece_ok() -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        let block = vec![0, 1, 2, 3];
        assert!(send_piece(&mut sender_stream, 0, 4, block.clone()).is_ok());

        let received_msg = receive_message(&mut receptor_stream)?;
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
        let _listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;

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
        let _listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;

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
        let listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;
        let (mut receptor_stream, _addr) = listener.accept()?;

        assert!(send_cancel(&mut sender_stream, 0, 4, 4).is_ok());

        let received_msg = receive_message(&mut receptor_stream)?;
        let expected_msg = P2PMessage::Cancel {
            piece_index: 0,
            beginning_byte_index: 4,
            amount_of_bytes: 4,
        };

        assert_eq!(expected_msg, received_msg);

        Ok(())
    }

    #[test]
    fn client_peer_send_cancel_of_zero_bytes_error() -> Result<(), Box<dyn Error>> {
        let _listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;

        assert_eq!(
            Err(MsgSenderError::ZeroAmountOfBytes(
                "[MsgSenderError] The amount of bytes cannot be equal zero.".to_string(),
            )),
            send_cancel(&mut sender_stream, 0, 4, 0)
        );

        Ok(())
    }

    #[test]
    fn client_peer_send_cancel_bigger_than_the_max_block_bytes_error() -> Result<(), Box<dyn Error>>
    {
        let _listener = TcpListener::bind(DEFAULT_ADDR)?;
        let mut sender_stream = TcpStream::connect(DEFAULT_ADDR)?;

        assert_eq!(
            Err(MsgSenderError::AmountOfBytesLimitExceeded(
                "[MsgSenderError] The amount of bytes must be smaller than 2^17.".to_string(),
            )),
            send_cancel(&mut sender_stream, 0, 4, MAX_BLOCK_BYTES + 1)
        );

        Ok(())
    }
}
