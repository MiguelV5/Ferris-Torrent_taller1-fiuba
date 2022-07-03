//! # Modulo de data de respuesta de un Tracker
//! Este modulo contiene las funciones encargadas de analizar y almacenar la
//! información importante tras haberse comunicado con un tracker

use crate::torrent::parsers::bencoding::values::ValuesBencoding;

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

type DicValues = HashMap<Vec<u8>, ValuesBencoding>;
type ResultResponse<T> = Result<T, ResponseError>;

const ZERO: u64 = 0;
const INTERVAL: &str = "interval";
const COMPLETE: &str = "complete";
const INCOMPLETE: &str = "incomplete";
const PEERS: &str = "peers";
const PEER_ID: &str = "peer id";
const IP: &str = "ip";
const PORT: &str = "port";

#[derive(Debug, PartialEq)]
///Enumerado que representa la seccion en la que el error puede surgir
pub enum Section {
    Interval,
    Complete,
    Incomplete,
    Peers,
    PeerId,
    Ip,
    Port,
}

#[derive(Debug, PartialEq)]
///Enumerado que representa la seccion en la que el error puede surgir al analizar una response
pub enum ResponseError {
    NotFound(Section),
    Format(Section),
    ConvertIp(Section),
}

#[derive(PartialEq, Debug, Clone)]
pub struct PeerDataFromTrackerResponse {
    pub peer_id: Option<Vec<u8>>,
    pub peer_address: SocketAddr,
}

fn to_sock_addr(ip: String, port: u16) -> ResultResponse<SocketAddr> {
    let ip_addr = match IpAddr::from_str(&ip) {
        Ok(ip_result) => ip_result,
        Err(_) => return Err(ResponseError::ConvertIp(Section::Ip)),
    };
    let result_addr = SocketAddr::new(ip_addr, port);
    Ok(result_addr)
}

fn decode_compact_peer(compact_peer: Vec<u8>) -> ResultResponse<(IpAddr, u16)> {
    let multiplier = 256;
    if compact_peer.len() != 6 {
        return Err(ResponseError::ConvertIp(Section::Ip));
    }
    let ipv4_addr = Ipv4Addr::new(
        compact_peer[0],
        compact_peer[1],
        compact_peer[2],
        compact_peer[3],
    );
    let port = (compact_peer[4] as u16 * multiplier) + compact_peer[5] as u16;
    Ok((IpAddr::V4(ipv4_addr), port))
}

impl PeerDataFromTrackerResponse {
    pub fn new(id: Option<Vec<u8>>, ip: String, port: u16) -> ResultResponse<Self> {
        let peer_id = id;
        let peer_address = to_sock_addr(ip, port)?;

        Ok(PeerDataFromTrackerResponse {
            peer_id,
            peer_address,
        })
    }

    pub fn new_from_compact(compact_peer: Vec<u8>) -> ResultResponse<Self> {
        let peer_id = None;
        let (ip_addr, port) = decode_compact_peer(compact_peer)?;
        let peer_address = SocketAddr::new(ip_addr, port);
        Ok(PeerDataFromTrackerResponse {
            peer_id,
            peer_address,
        })
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct TrackerResponseData {
    pub interval: u64,
    pub complete: u64,
    pub incomplete: u64,
    pub peers: Vec<PeerDataFromTrackerResponse>,
}

fn vec_u8_to_string(vec: &[u8]) -> String {
    String::from_utf8_lossy(vec).into_owned()
}

fn get_dic_u64(dic_res: &DicValues, value: &str, section: Section) -> ResultResponse<u64> {
    match dic_res.get(&value.as_bytes().to_vec()) {
        Some(ValuesBencoding::Integer(num_value)) => Ok(*num_value as u64),
        Some(_) => Err(ResponseError::Format(section)),
        None => Err(ResponseError::NotFound(section)),
    }
}

fn get_dic_string(dic_res: &DicValues, value: &str, section: Section) -> ResultResponse<Vec<u8>> {
    match dic_res.get(&value.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(str_value)) => Ok(str_value.clone()),
        Some(_) => Err(ResponseError::Format(section)),
        None => Err(ResponseError::NotFound(section)),
    }
}

fn init_peers(dic_response: &DicValues) -> ResultResponse<Vec<PeerDataFromTrackerResponse>> {
    let mut vector_peers = vec![];

    match dic_response.get(&PEERS.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list_peers)) => {
            for peer in list_peers {
                match peer {
                    ValuesBencoding::Dic(dic_peer) => {
                        let ip = get_dic_string(dic_peer, IP, Section::Ip)?;
                        let ip = vec_u8_to_string(&ip);
                        let port = get_dic_u64(dic_peer, PORT, Section::Port)?;
                        let peer_id = match get_dic_string(dic_peer, PEER_ID, Section::PeerId) {
                            Ok(id) => Some(id),
                            Err(ResponseError::NotFound(_)) => None,
                            Err(error) => return Err(error),
                        };
                        let peer_struct =
                            PeerDataFromTrackerResponse::new(peer_id, ip, port as u16)?;
                        vector_peers.push(peer_struct);
                    }
                    _ => return Err(ResponseError::Format(Section::Peers)),
                }
            }
            Ok(vector_peers)
        }
        Some(ValuesBencoding::String(peers_compact)) => {
            let long_compact = 6;
            let list_peers_compact: Vec<Vec<u8>> = peers_compact
                .chunks(long_compact)
                .map(|s| s.into())
                .collect();
            for peer_compact in list_peers_compact {
                if let Ok(peer_struct) = PeerDataFromTrackerResponse::new_from_compact(peer_compact)
                {
                    vector_peers.push(peer_struct)
                };
            }
            Ok(vector_peers)
        }
        Some(_) => Err(ResponseError::Format(Section::Peers)),
        None => Err(ResponseError::NotFound(Section::Peers)),
    }
}

impl TrackerResponseData {
    pub fn new(dic_response: DicValues) -> Result<Self, ResponseError> {
        let interval = get_dic_u64(&dic_response, INTERVAL, Section::Interval)?;

        let complete = match get_dic_u64(&dic_response, COMPLETE, Section::Complete) {
            Ok(value_complete) => value_complete,
            Err(ResponseError::NotFound(_)) => ZERO,
            Err(error) => return Err(error),
        };
        let incomplete = match get_dic_u64(&dic_response, INCOMPLETE, Section::Incomplete) {
            Ok(value_incomplete) => value_incomplete,
            Err(ResponseError::NotFound(_)) => ZERO,
            Err(error) => return Err(error),
        };
        let peers = init_peers(&dic_response)?;

        Ok(TrackerResponseData {
            interval,
            complete,
            incomplete,
            peers,
        })
    }

    pub fn get_peer_address(&self, peer_index: usize) -> Option<SocketAddr> {
        self.peers
            .get(peer_index)
            .map(|peer_data| peer_data.peer_address)
    }

    pub fn has_expected_peer_id(&self, peer_index: usize, peer_id: &[u8]) -> bool {
        if let Some(peer_data) = self.peers.get(peer_index) {
            if let Some(expected_peer_id) = &peer_data.peer_id {
                expected_peer_id == peer_id
            } else {
                true
            }
        } else {
            false
        }
    }

    pub fn get_total_amount_peers(&self) -> usize {
        self.peers.len()
    }

    pub fn get_total_amount_leechers(&self) -> u64 {
        self.incomplete
    }

    pub fn get_total_amount_seeders(&self) -> u64 {
        self.complete
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_get_dic_u64_ok() {
        let mut dic = HashMap::new();
        dic.insert("info".as_bytes().to_vec(), ValuesBencoding::Integer(125));
        dic.insert("port".as_bytes().to_vec(), ValuesBencoding::Integer(5091));
        dic.insert(
            "interval".as_bytes().to_vec(),
            ValuesBencoding::Integer(999),
        );
        dic.insert(
            "complete".as_bytes().to_vec(),
            ValuesBencoding::Integer(10123),
        );
        dic.insert("final".as_bytes().to_vec(), ValuesBencoding::Integer(6969));

        let interval = get_dic_u64(&dic, INTERVAL, Section::Interval);
        let complete = get_dic_u64(&dic, COMPLETE, Section::Complete);
        let interval_expected = 999;
        let complete_expected = 10123;

        assert_eq!(interval, Ok(interval_expected));
        assert_eq!(complete, Ok(complete_expected));
    }
    #[test]
    fn test_get_dic_u64_error_format() {
        let mut dic = HashMap::new();
        dic.insert(
            "port".as_bytes().to_vec(),
            ValuesBencoding::String("8081".as_bytes().to_vec()),
        );

        let port = get_dic_u64(&dic, PORT, Section::Port);

        assert_eq!(port, Err(ResponseError::Format(Section::Port)))
    }
    #[test]
    fn test_get_dic_u64_error_not_found() {
        let mut dic = HashMap::new();
        dic.insert(
            "port".as_bytes().to_vec(),
            ValuesBencoding::String("8081".as_bytes().to_vec()),
        );

        let port = get_dic_u64(&dic, INTERVAL, Section::Interval);

        assert_eq!(port, Err(ResponseError::NotFound(Section::Interval)))
    }
    #[test]
    fn test_get_dic_string_ok() {
        let mut dic = HashMap::new();
        dic.insert(
            "info".as_bytes().to_vec(),
            ValuesBencoding::String("information".as_bytes().to_vec()),
        );
        dic.insert(
            "port".as_bytes().to_vec(),
            ValuesBencoding::String("8080".as_bytes().to_vec()),
        );
        dic.insert(
            "interval".as_bytes().to_vec(),
            ValuesBencoding::String("an interval".as_bytes().to_vec()),
        );
        dic.insert(
            "complete".as_bytes().to_vec(),
            ValuesBencoding::String("19.99.129".as_bytes().to_vec()),
        );
        dic.insert(
            "final".as_bytes().to_vec(),
            ValuesBencoding::String("FIN".as_bytes().to_vec()),
        );

        let port = get_dic_string(&dic, PORT, Section::Port);
        let complete = get_dic_string(&dic, COMPLETE, Section::Complete);
        let port_expected = "8080".as_bytes().to_vec();
        let complete_expected = "19.99.129".as_bytes().to_vec();

        assert_eq!(port, Ok(port_expected));
        assert_eq!(complete, Ok(complete_expected));
    }
    #[test]
    fn test_get_dic_string_error_format() {
        let mut dic = HashMap::new();
        dic.insert("port".as_bytes().to_vec(), ValuesBencoding::Integer(8081));

        let port = get_dic_string(&dic, PORT, Section::Port);

        assert_eq!(port, Err(ResponseError::Format(Section::Port)))
    }
    #[test]
    fn test_get_dic_string_error_not_found() {
        let mut dic = HashMap::new();
        dic.insert(
            "port".as_bytes().to_vec(),
            ValuesBencoding::String("8081".as_bytes().to_vec()),
        );

        let port = get_dic_string(&dic, INTERVAL, Section::Interval);

        assert_eq!(port, Err(ResponseError::NotFound(Section::Interval)))
    }

    //    #[test]
    //    fn test_from_str_to_ipaddr_ok() {
    //        let ip = String::from("197.0.12.1");
    //        let ipaddr_expected = IpAddr::V4(Ipv4Addr::new(197, 0, 12, 1));
    //
    //        assert_eq!(Ok(ipaddr_expected), from_str_to_ipaddr(ip))
    //    }
    //    #[test]
    //    fn test_from_str_to_ipaddr_error_lenght() {
    //        let ip = String::from("8081");
    //        assert_eq!(
    //            Err(ResponseError::ConvertIp(Section::Ip)),
    //            from_str_to_ipaddr(ip)
    //        );
    //
    //        let ip = String::from("197.0.0.1.9");
    //        assert_eq!(
    //            Err(ResponseError::ConvertIp(Section::Ip)),
    //            from_str_to_ipaddr(ip)
    //        );
    //
    //        let ip = String::from("177.1.12");
    //        assert_eq!(
    //            Err(ResponseError::ConvertIp(Section::Ip)),
    //            from_str_to_ipaddr(ip)
    //        );
    //    }
    //    #[test]
    //    fn test_from_str_to_ipaddr_error_numbers() {
    //        let ip = String::from("12.0.0.1.a");
    //        assert_eq!(
    //            Err(ResponseError::ConvertIp(Section::Ip)),
    //            from_str_to_ipaddr(ip)
    //        );
    //
    //        let ip = String::from("A.B.C.D.E");
    //        assert_eq!(
    //            Err(ResponseError::ConvertIp(Section::Ip)),
    //            from_str_to_ipaddr(ip)
    //        );
    //    }
}
