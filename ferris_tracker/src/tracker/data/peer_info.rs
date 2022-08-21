use super::constants::*;
use shared::parsers::urlencoding;
use std::{error::Error, fmt, net::SocketAddr};

#[derive(PartialEq, Eq, Debug)]
pub enum Event {
    Started,
    Completed,
    Stopped,
}

#[derive(PartialEq, Eq, Debug)]
pub enum PeerInfoError {
    InfoHashNotFound,
    InfoHashInvalid,
    PeerIdNotFoundOrInvalid,
    PortNotFound,
    PortInvalid,
    StatNotFound,
    StatInvalid,
    PoissonedLock,
}

impl fmt::Display for PeerInfoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for PeerInfoError {}

#[derive(PartialEq, Eq, Debug)]
pub struct PeerInfo {
    //INGRESADO AL CREAR
    sock_addr: SocketAddr,
    //OBLIGATORIOS DE ANNOUNCE
    info_hash: Vec<u8>,
    peer_id: Vec<u8>,
    port: u64,
    downloaded: u64,
    uploaded: u64,
    left: u64,
    //OPCIONALES DE ANNOUNCE
    compact: Option<Vec<u8>>,
    event: Option<Event>,
}

fn find_index_msg(response: &[u8], size: usize, end_line: &[u8]) -> Option<usize> {
    let first_pos = response.windows(size).position(|arr| arr == end_line);
    first_pos.map(|pos| pos + size)
}

fn take_result(announce: &[u8]) -> Vec<u8> {
    let mut result = vec![];
    for &char in announce {
        if char == b'&' || char == b' ' {
            break;
        }
        result.push(char);
    }
    result
}

fn obtain_value_from_querystring(
    announce: &[u8],
    size_command: usize,
    command: &str,
) -> Option<Vec<u8>> {
    let pos_result = find_index_msg(announce, size_command, command.as_bytes());
    pos_result.map(|pos| take_result(&announce[pos..]))
}

fn from_vec_to_port(result: Option<Vec<u8>>) -> Result<u64, PeerInfoError> {
    match result {
        Some(vec) => {
            let str_port = String::from_utf8_lossy(&vec).to_string();
            if let Ok(port_num) = str_port.parse::<u64>() {
                if RANGE_PORT.contains(&port_num) {
                    Ok(port_num)
                } else {
                    Err(PeerInfoError::PortInvalid)
                }
            } else {
                Err(PeerInfoError::PortInvalid)
            }
        }
        None => Err(PeerInfoError::PortNotFound),
    }
}

fn get_event(name_event: String) -> Option<Event> {
    match name_event {
        _ if name_event == STARTED => Some(Event::Started),
        _ if name_event == COMPLETED => Some(Event::Completed),
        _ if name_event == STOPPED => Some(Event::Stopped),
        _ => None,
    }
}

fn obtain_info_hash_from_querystring(announce: &[u8]) -> Result<Vec<u8>, PeerInfoError> {
    match obtain_value_from_querystring(announce, INFO_HASH.len(), INFO_HASH) {
        Some(info_hash_url) => {
            let url_decoded = urlencoding::decoder::from_url(info_hash_url);
            if url_decoded.len() != 20 {
                Err(PeerInfoError::InfoHashInvalid)
            } else {
                Ok(url_decoded)
            }
        }
        None => Err(PeerInfoError::InfoHashNotFound),
    }
}

fn obtain_peer_id_from_querystring(announce: &[u8]) -> Result<Vec<u8>, PeerInfoError> {
    match obtain_value_from_querystring(announce, PEER_ID.len(), PEER_ID) {
        Some(peer_id_url) => {
            let url_decoded = urlencoding::decoder::from_url(peer_id_url);
            if url_decoded.len() != 20 {
                Err(PeerInfoError::PeerIdNotFoundOrInvalid)
            } else {
                Ok(url_decoded)
            }
        }
        None => Err(PeerInfoError::PeerIdNotFoundOrInvalid),
    }
}

fn obtain_port_from_querystring(announce: &[u8]) -> Result<u64, PeerInfoError> {
    let port = obtain_value_from_querystring(announce, PORT.len(), PORT);
    from_vec_to_port(port)
}

fn obtain_stat_from_querystring(announce: &[u8], stat_type: &str) -> Result<u64, PeerInfoError> {
    let stat = obtain_value_from_querystring(announce, stat_type.len(), stat_type);
    match stat {
        Some(vec) => {
            let str_num = String::from_utf8_lossy(&vec).to_string();
            match str_num.parse::<u64>() {
                Ok(number_res) => Ok(number_res),
                Err(_) => Err(PeerInfoError::StatInvalid),
            }
        }
        None => Err(PeerInfoError::StatNotFound),
    }
}

fn obtain_event_from_querystring(announce: &[u8]) -> Option<Event> {
    match obtain_value_from_querystring(announce, EVENT.len(), EVENT) {
        Some(vector_event) => match String::from_utf8(vector_event) {
            Ok(value) => get_event(value),
            Err(_) => None,
        },
        None => None,
    }
}

fn obtain_compact_from_querystring(announce: &[u8]) -> Option<Vec<u8>> {
    obtain_value_from_querystring(announce, COMPACT.len(), COMPACT)
}

impl PeerInfo {
    pub fn get_info_hash(&self) -> Vec<u8> {
        self.info_hash.clone()
    }

    pub fn get_peer_id(&self) -> Vec<u8> {
        self.peer_id.clone()
    }

    pub fn get_port(&self) -> u64 {
        self.port
    }

    pub fn get_sock_addr(&self) -> SocketAddr {
        self.sock_addr
    }

    pub fn get_downloaded_uploaded(&self) -> (u64, u64) {
        (self.downloaded, self.uploaded)
    }

    pub fn is_complete(&self) -> bool {
        if let Some(Event::Completed) = self.event {
            return true;
        };
        self.left == ZERO
    }

    pub fn is_stopped(&self) -> bool {
        matches!(self.event, Some(Event::Stopped))
    }

    pub fn is_compact(&self) -> bool {
        match self.compact.clone() {
            Some(mut value) => match value.pop() {
                Some(num) => num == b'1',
                None => false,
            },
            None => false,
        }
    }

    pub fn new(announce: Vec<u8>, sock_addr: SocketAddr) -> Result<Self, PeerInfoError> {
        let mut sock_addr = sock_addr;
        //Si uno de los campos obligatorios del Announce no existe devuelvo error
        let info_hash = match obtain_info_hash_from_querystring(&announce) {
            Ok(result) => result,
            Err(error) => return Err(error),
        };
        let peer_id = match obtain_peer_id_from_querystring(&announce) {
            Ok(result) => result,
            Err(error) => return Err(error),
        };
        let port = match obtain_port_from_querystring(&announce) {
            Ok(result) => result,
            Err(error) => return Err(error),
        };
        let downloaded = match obtain_stat_from_querystring(&announce, DOWNLOADED) {
            Ok(result) => result,
            Err(error) => return Err(error),
        };
        let uploaded = match obtain_stat_from_querystring(&announce, UPLOADED) {
            Ok(result) => result,
            Err(error) => return Err(error),
        };
        let left = match obtain_stat_from_querystring(&announce, LEFT) {
            Ok(result) => result,
            Err(error) => return Err(error),
        };
        let compact = obtain_compact_from_querystring(&announce);
        let event = obtain_event_from_querystring(&announce);

        //Cambio el puerto dado por el que me dieron en el announce
        sock_addr.set_port(port as u16);

        Ok(PeerInfo {
            sock_addr,
            info_hash,
            peer_id,
            port,
            downloaded,
            uploaded,
            left,
            compact,
            event,
        })
    }
}

pub fn get_error_response_for_announce(error: PeerInfoError) -> String {
    match error {
        PeerInfoError::InfoHashNotFound => ERROR_INFO_HASH_NOT_FOUND.to_owned(),
        PeerInfoError::InfoHashInvalid => ERROR_INFO_HASH_INVALID.to_owned(),
        PeerInfoError::PeerIdNotFoundOrInvalid => ERROR_PEER_ID_INVALID.to_owned(),
        PeerInfoError::StatNotFound => ERROR_STAT_NOT_FOUND.to_owned(),
        PeerInfoError::StatInvalid => ERROR_STAT_INVALID.to_owned(),
        PeerInfoError::PortNotFound => ERROR_STAT_NOT_FOUND.to_owned(),
        PeerInfoError::PortInvalid => ERROR_PORT_INVALID.to_owned(),
        PeerInfoError::PoissonedLock => ERROR_500.to_owned(),
    }
}

#[cfg(test)]
mod tests_peer_info {
    use super::*;
    use std::{net::SocketAddr, str::FromStr};

    use crate::{
        tracker::data::peer_info::{get_event, PeerInfo},
        ResultDyn,
    };

    mod test_obtaining_peer_info_from_incomplete_announce {
        use super::*;

        #[test]
        fn obtaining_peer_info_without_info_hash_returns_err() -> ResultDyn<()> {
            let initial_addr = SocketAddr::from_str("127.0.0.1:9999")?;
            let peer_id_str = "ABCDEFGHIJKLMNOPQRST";
            let ip_str = "127.0.0.1";
            let port_str = "6881";
            let uploaded_str = "0";
            let downloaded_str = "0";
            let left_str = "128";
            let compact_str = "1";
            let event_str = "started";

            let get_announce_msg = format!("GET /announce?peer_id={}&ip={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", peer_id_str, ip_str, port_str, uploaded_str, downloaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

            assert_eq!(
                PeerInfo::new(get_announce_msg, initial_addr),
                Err(PeerInfoError::InfoHashNotFound)
            );

            Ok(())
        }

        #[test]
        fn obtaining_peer_info_without_peer_id_returns_err() -> ResultDyn<()> {
            let initial_addr = SocketAddr::from_str("127.0.0.1:9999")?;
            let info_hash_str = "abcdefghijklmn123456";
            let ip_str = "127.0.0.1";
            let port_str = "6881";
            let uploaded_str = "0";
            let downloaded_str = "0";
            let left_str = "128";
            let compact_str = "1";
            let event_str = "started";

            let get_announce_msg = format!("GET /announce?info_hash={}&ip={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, ip_str, port_str, uploaded_str, downloaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

            assert_eq!(
                PeerInfo::new(get_announce_msg, initial_addr),
                Err(PeerInfoError::PeerIdNotFoundOrInvalid)
            );

            Ok(())
        }

        #[test]
        fn obtaining_peer_info_without_port_returns_err() -> ResultDyn<()> {
            let initial_addr = SocketAddr::from_str("127.0.0.1:9999")?;
            let info_hash_str = "abcdefghijklmn123456";
            let peer_id_str = "ABCDEFGHIJKLMNOPQRST";
            let ip_str = "127.0.0.1";
            let uploaded_str = "0";
            let downloaded_str = "0";
            let left_str = "128";
            let compact_str = "1";
            let event_str = "started";

            let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&uploaded={}&downloaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str,  uploaded_str, downloaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

            assert_eq!(
                PeerInfo::new(get_announce_msg, initial_addr),
                Err(PeerInfoError::PortNotFound)
            );

            Ok(())
        }

        #[test]
        fn obtaining_peer_info_without_stats_returns_err() -> ResultDyn<()> {
            let initial_addr = SocketAddr::from_str("127.0.0.1:9999")?;
            let info_hash_str = "abcdefghijklmn123456";
            let peer_id_str = "ABCDEFGHIJKLMNOPQRST";
            let ip_str = "127.0.0.1";
            let port_str = "6881";
            let uploaded_str = "0";
            let downloaded_str = "0";
            let left_str = "128";
            let compact_str = "1";
            let event_str = "started";

            let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&port={}&downloaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str, port_str,  downloaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

            assert_eq!(
                PeerInfo::new(get_announce_msg, initial_addr),
                Err(PeerInfoError::StatNotFound)
            );

            let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&port={}&uploaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str, port_str, uploaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

            assert_eq!(
                PeerInfo::new(get_announce_msg, initial_addr),
                Err(PeerInfoError::StatNotFound)
            );

            let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&port={}&uploaded={}&downloaded={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str, port_str, uploaded_str, downloaded_str, event_str, compact_str).as_bytes().to_vec();

            assert_eq!(
                PeerInfo::new(get_announce_msg, initial_addr),
                Err(PeerInfoError::StatNotFound)
            );

            Ok(())
        }
    }

    #[test]
    fn obtaining_peer_info_from_valid_announce_ok() -> ResultDyn<()> {
        let initial_addr = SocketAddr::from_str("127.0.0.1:9999")?;
        let info_hash_str = "abcdefghijklmn123456";
        let peer_id_str = "ABCDEFGHIJKLMNOPQRST";
        let ip_str = "127.0.0.1";
        let port_str = "6881";
        let uploaded_str = "0";
        let downloaded_str = "0";
        let left_str = "128";
        let compact_str = "1";
        let event_str = "started";

        let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&port={}&uploaded={}&downloaded={}&left={}&event={}&compact={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str, port_str, uploaded_str, downloaded_str, left_str, event_str, compact_str).as_bytes().to_vec();

        let mut expected_addr = initial_addr.clone();
        expected_addr.set_port(u16::from_str(port_str)?);
        let expected_info_hash = info_hash_str.as_bytes().to_vec();
        let expected_peer_id = peer_id_str.as_bytes().to_vec();
        let expected_port = u64::from_str(port_str)?;
        let expected_uploaded = u64::from_str(uploaded_str)?;
        let expected_downloaded = u64::from_str(downloaded_str)?;
        let expected_left = u64::from_str(left_str)?;
        let expected_compact = compact_str.as_bytes().to_vec();
        let expected_event = get_event(event_str.to_string());

        assert_eq!(
            PeerInfo::new(get_announce_msg, initial_addr),
            Ok(PeerInfo {
                sock_addr: expected_addr,
                info_hash: expected_info_hash.clone(),
                peer_id: expected_peer_id.clone(),
                port: expected_port,
                downloaded: expected_downloaded,
                uploaded: expected_uploaded,
                left: expected_left,
                compact: Some(expected_compact.clone()),
                event: expected_event,
            })
        );

        // Con el querystring en distinto orden:

        let get_announce_msg = format!("GET /announce?compact={}&peer_id={}&ip={}&uploaded={}&event={}&downloaded={}&left={}&port={}&info_hash={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", compact_str, peer_id_str, ip_str, uploaded_str, event_str, downloaded_str, left_str, port_str, info_hash_str).as_bytes().to_vec();

        let expected_event = get_event(event_str.to_string());

        assert_eq!(
            PeerInfo::new(get_announce_msg, initial_addr),
            Ok(PeerInfo {
                sock_addr: expected_addr,
                info_hash: expected_info_hash,
                peer_id: expected_peer_id,
                port: expected_port,
                downloaded: expected_downloaded,
                uploaded: expected_uploaded,
                left: expected_left,
                compact: Some(expected_compact),
                event: expected_event,
            })
        );

        Ok(())
    }

    #[test]
    fn obtaining_peer_info_from_valid_announce_without_optional_values_ok() -> ResultDyn<()> {
        let initial_addr = SocketAddr::from_str("127.0.0.1:9999")?;
        let info_hash_str = "abcdefghijklmn123456";
        let peer_id_str = "ABCDEFGHIJKLMNOPQRST";
        let ip_str = "127.0.0.1";
        let port_str = "6881";
        let uploaded_str = "0";
        let downloaded_str = "0";
        let left_str = "128";

        let get_announce_msg = format!("GET /announce?info_hash={}&peer_id={}&ip={}&port={}&uploaded={}&downloaded={}&left={} HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n", info_hash_str, peer_id_str, ip_str, port_str, uploaded_str, downloaded_str, left_str).as_bytes().to_vec();

        let mut expected_addr = initial_addr.clone();
        expected_addr.set_port(u16::from_str(port_str)?);
        let expected_info_hash = info_hash_str.as_bytes().to_vec();
        let expected_peer_id = peer_id_str.as_bytes().to_vec();
        let expected_port = u64::from_str(port_str)?;
        let expected_uploaded = u64::from_str(uploaded_str)?;
        let expected_downloaded = u64::from_str(downloaded_str)?;
        let expected_left = u64::from_str(left_str)?;

        assert_eq!(
            PeerInfo::new(get_announce_msg, initial_addr),
            Ok(PeerInfo {
                sock_addr: expected_addr,
                info_hash: expected_info_hash,
                peer_id: expected_peer_id,
                port: expected_port,
                downloaded: expected_downloaded,
                uploaded: expected_uploaded,
                left: expected_left,
                compact: None,
                event: None,
            })
        );

        Ok(())
    }
}
