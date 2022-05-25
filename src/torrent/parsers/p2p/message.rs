use std::{error::Error, fmt};

#[derive(PartialEq, Debug, Clone)]
/// Representa el estado de una pieza para uso en mensaje P2P Bitfield
pub enum PieceStatus {
    ValidAndAvailablePiece,
    MissingPiece,
}

#[derive(PartialEq, Debug, Clone)]
/// Representa un mensaje de comunicación P2P.
pub enum P2PMessage {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {
        piece_index: u32,
    },
    Bitfield {
        bitfield: Vec<PieceStatus>,
    },
    Request {
        piece_index: u32,
        beginning_byte_index: u32,
        amount_of_bytes: u32,
    },
    Piece {
        piece_index: u32,
        beginning_byte_index: u32,
        block: Vec<u8>,
    },
    Cancel {
        piece_index: u32,
        beginning_byte_index: u32,
        amount_of_bytes: u32,
    },
    Port {
        listen_port: u32,
    },
    Handshake {
        protocol_str: String,
        info_hash: Vec<u8>, // Valor del SHA1
        peer_id: String,
    },
}

#[derive(PartialEq, Debug)]
/// Representa un tipo de error en la DECODIFICACION de mensajes P2P
pub enum P2PMessageDecodingError {
    ByteAmountError(String),
    FromUsizeToU32Error(String),
    FromBytesToStringError(String),
    InvalidIdError(String),
    InvalidProtocolStrError(String),
}

impl fmt::Display for P2PMessageDecodingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for P2PMessageDecodingError {}

#[derive(PartialEq, Debug)]
/// Representa un tipo de error en la ENCODIFICACION de mensajes P2P
pub enum P2PMessageEncodingError {
    FromUsizeToU32Error(String),
    InvalidProtocolStrError(String),
}

impl fmt::Display for P2PMessageEncodingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for P2PMessageEncodingError {}
