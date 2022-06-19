//! # Modulo de constantes
//! Constantes utiles para uso en comunicacion http

pub const ONE: usize = 1;
pub const TWO: usize = 2;
pub const THREE: usize = 3;
pub const FOUR: usize = 4;

pub const LAST_SLASH: &[u8; ONE] = b"/";
pub const HTTP_END: &[u8; THREE] = b"://";
pub const TWO_POINTS: &[u8; ONE] = b":";
pub const END_LINE: &[u8; TWO] = b"\r\n";
pub const DOUBLE_END_LINE: &[u8; FOUR] = b"\r\n\r\n";

pub const INIT_MSG: &str = "GET /announce";
pub const INFO_HASH: &str = "?info_hash=";
pub const PEER_ID: &str = "&peer_id=";
pub const IP: &str = "&ip=";
pub const PORT: &str = "&port=";
pub const UPLOADED: &str = "&uploaded=";
pub const DOWNLOADED: &str = "&downloaded=";

pub const LEFT: &str = "&left=";
pub const EVENT: &str = "&event=";
pub const HTTP: &str = " HTTP/1.0\r\n";
pub const HOST: &str = "Host:";
pub const MSG_ENDING: &str = "\r\n\r\n";

pub const IP_CLIENT: &str = "127.0.0.1";
pub const PORT_HTTPS: &str = ":443";
pub const STARTED: &str = "started";
//pub const INIT_PORT: u32 = 6881;
