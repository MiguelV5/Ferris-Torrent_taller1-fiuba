#![allow(dead_code)]

use super::super::parsers::bencoding::values::ValuesBencoding;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

type DicValues = HashMap<Vec<u8>, ValuesBencoding>;
type ResultResponse<T> = Result<T, ResponseError>;

const INTERVAL: &str = "interval";
const COMPLETE: &str = "complete";
const INCOMPLETE: &str = "incomplete";
const PEERS: &str = "peers";
const PEER_ID: &str = "peer id";
const IP: &str = "ip";
const PORT: &str = "port";

#[derive(Debug, PartialEq)]
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

fn ip_str_to_u8(string: &str) -> ResultResponse<u8> {
    match string.parse::<u8>() {
        Ok(number) => Ok(number),
        Err(_) => Err(ResponseError::ConvertIp(Section::Ip)),
    }
}

fn from_str_to_ipaddr(ip: String) -> ResultResponse<IpAddr> {
    let iter_numbers = ip.split('.');
    let mut vec_numbers = vec![];

    for number in iter_numbers {
        vec_numbers.push(ip_str_to_u8(number)?)
    }

    if vec_numbers.len() != 4 {
        return Err(ResponseError::ConvertIp(Section::Ip));
    }

    let ipv4_addr = Ipv4Addr::new(
        vec_numbers[0],
        vec_numbers[1],
        vec_numbers[2],
        vec_numbers[3],
    );
    Ok(IpAddr::V4(ipv4_addr))
}

impl PeerDataFromTrackerResponse {
    pub fn new(id: Vec<u8>, ip: String, port: u16) -> ResultResponse<Self> {
        let mut peer_id = None;
        let ip_addr = from_str_to_ipaddr(ip)?;
        let peer_address = SocketAddr::new(ip_addr, port);
        if !id.is_empty() {
            peer_id = Some(id);
        }

        Ok(PeerDataFromTrackerResponse {
            peer_id,
            peer_address,
        })
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct TrackerResponseData {
    pub interval: u64,
    //pub tracker_id: String,
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
                        let peer_id = get_dic_string(dic_peer, PEER_ID, Section::PeerId)?;
                        let peer_struct =
                            PeerDataFromTrackerResponse::new(peer_id, ip, port as u16)?;
                        vector_peers.push(peer_struct);
                    }
                    _ => return Err(ResponseError::Format(Section::Peers)),
                }
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
        let complete = get_dic_u64(&dic_response, COMPLETE, Section::Complete)?;
        let incomplete = get_dic_u64(&dic_response, INCOMPLETE, Section::Incomplete)?;
        let peers = init_peers(&dic_response)?;

        Ok(TrackerResponseData {
            interval,
            complete,
            incomplete,
            peers,
        })
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
    #[test]
    fn test_from_str_to_ipaddr_ok() {
        let ip = String::from("197.0.12.1");
        let ipaddr_expected = IpAddr::V4(Ipv4Addr::new(197, 0, 12, 1));

        assert_eq!(Ok(ipaddr_expected), from_str_to_ipaddr(ip))
    }
    #[test]
    fn test_from_str_to_ipaddr_error_lenght() {
        let ip = String::from("8081");
        assert_eq!(
            Err(ResponseError::ConvertIp(Section::Ip)),
            from_str_to_ipaddr(ip)
        );

        let ip = String::from("197.0.0.1.9");
        assert_eq!(
            Err(ResponseError::ConvertIp(Section::Ip)),
            from_str_to_ipaddr(ip)
        );

        let ip = String::from("177.1.12");
        assert_eq!(
            Err(ResponseError::ConvertIp(Section::Ip)),
            from_str_to_ipaddr(ip)
        );
    }
    #[test]
    fn test_from_str_to_ipaddr_error_numbers() {
        let ip = String::from("12.0.0.1.a");
        assert_eq!(
            Err(ResponseError::ConvertIp(Section::Ip)),
            from_str_to_ipaddr(ip)
        );

        let ip = String::from("A.B.C.D.E");
        assert_eq!(
            Err(ResponseError::ConvertIp(Section::Ip)),
            from_str_to_ipaddr(ip)
        );
    }
}
