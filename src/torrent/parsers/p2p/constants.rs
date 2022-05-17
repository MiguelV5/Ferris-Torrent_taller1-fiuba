pub const NUM_BITS_ON_A_BYTE: usize = 8;

pub const MIN_BYTES_OF_A_P2P_MSG: usize = 4;
pub const NEEDED_NUM_OF_BYTES_TO_CONCATENATE: usize = 4;
pub const NUM_OF_BYTES_LENGHT_PREFIX: usize = 4;

pub const TOTAL_NUM_OF_BYTES_HANDSHAKE: usize = 68;
pub const PSTRLEN_VALUE_HANDSHAKE: u8 = 19;
pub const PSTR_STRING_HANDSHAKE: &str = "BitTorrent protocol";

pub const NEEDED_NUM_OF_BYTES_FOR_ID: u32 = 1;
pub const ID_CHOKE: u8 = 0;
pub const ID_UNCHOKE: u8 = 1;
pub const ID_INTERESTED: u8 = 2;
pub const ID_NOT_INTERESTED: u8 = 3;
pub const ID_HAVE: u8 = 4;
pub const ID_BITFIELD: u8 = 5;
pub const ID_REQUEST: u8 = 6;
pub const ID_PIECE: u8 = 7;
pub const ID_CANCEL: u8 = 8;
pub const ID_PORT: u8 = 9;

//pub const DEFAULT_REQUESTED_BLOCK_SIZE: u32 = 16384; // 16KB - 2^14.
