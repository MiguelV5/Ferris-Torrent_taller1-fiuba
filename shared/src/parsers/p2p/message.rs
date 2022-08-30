use std::{error::Error, fmt};

#[derive(PartialEq, Eq, Debug, Clone)]
/// Representa el estado de una pieza para uso en mensaje P2P Bitfield
/// (Teniendo un vector de PieceStatuses se puede representar el Bitfield de forma comoda)
pub enum PieceStatus {
    ValidAndAvailablePiece,
    PartiallyDownloaded {
        downloaded_bytes: u32,
        was_requested: bool,
    },
    MissingPiece {
        was_requested: bool,
    },
}

#[derive(PartialEq, Eq, Debug, Clone)]
/// Representa un mensaje en general de comunicación P2P, donde cada variante es un mensaje distinto con
/// información asociada.
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
        peer_id: Vec<u8>,
    },
}

#[derive(PartialEq, Eq, Debug)]
/// Representa un tipo de error en la DECODIFICACION de mensajes P2P
pub enum P2PMessageDecodingError {
    ByteAmount(String),
    FromUsizeToU32(String),
    FromBytesToString(String),
    InvalidId(String),
    InvalidProtocolStr(String),
}

impl fmt::Display for P2PMessageDecodingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for P2PMessageDecodingError {}

#[derive(PartialEq, Eq, Debug)]
/// Representa un tipo de error en la ENCODIFICACION de mensajes P2P
pub enum P2PMessageEncodingError {
    FromUsizeToU32Error(String),
    InvalidProtocolStrError(String),
}

impl fmt::Display for P2PMessageEncodingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for P2PMessageEncodingError {}
