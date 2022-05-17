#[derive(PartialEq, Debug, Clone)]
/// Representa el estado de una pieza para uso en mensaje P2P Bitfield
pub enum PieceStatus {
    ValidAndAvailablePiece,
    MissingPiece,
}

#[derive(PartialEq, Debug)]
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
/// Representa un tipo de error en la interpretacion de mensajes P2P
pub enum P2PMessageError {
    ByteAmountError,
    FromUsizeToU32Error,
    FromBytesToStringError,
    InvalidIdError,
    InterpretationError,
    InvalidProtocolStrError,
}
