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
    pub peer_id: Option<String>,
    pub peer_address: SocketAddr,
}

fn str_to_u8(string: &str) -> ResultResponse<u8> {
    match string.parse::<u8>() {
        Ok(number) => Ok(number),
        Err(_) => Err(ResponseError::ConvertIp(Section::Ip)),
    }
}

fn from_str_to_ipaddr(ip: String) -> ResultResponse<IpAddr> {
    let mut iter_numbers = ip.split('.');

    let a = match iter_numbers.next() {
        Some(num) => str_to_u8(num)?,
        None => return Err(ResponseError::ConvertIp(Section::Ip)),
    };
    let b = match iter_numbers.next() {
        Some(num) => str_to_u8(num)?,
        None => return Err(ResponseError::ConvertIp(Section::Ip)),
    };
    let c = match iter_numbers.next() {
        Some(num) => str_to_u8(num)?,
        None => return Err(ResponseError::ConvertIp(Section::Ip)),
    };
    let d = match iter_numbers.next() {
        Some(num) => str_to_u8(num)?,
        None => return Err(ResponseError::ConvertIp(Section::Ip)),
    };

    let ipv4_addr = Ipv4Addr::new(a, b, c, d);
    Ok(IpAddr::V4(ipv4_addr))
}

impl PeerDataFromTrackerResponse {
    pub fn new(id: Vec<u8>, ip: String, port: u16) -> ResultResponse<Self> {
        let mut peer_id = None;
        let ip_addr = from_str_to_ipaddr(ip)?;
        let peer_address = SocketAddr::new(ip_addr, port);
        if !id.is_empty() {
            peer_id = Some(String::from_utf8_lossy(&id).to_string());
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

fn init_interval(dic_response: &DicValues) -> ResultResponse<u64> {
    match dic_response.get(&INTERVAL.as_bytes().to_vec()) {
        Some(ValuesBencoding::Integer(interval)) => Ok(*interval as u64),
        Some(_) => Err(ResponseError::Format(Section::Interval)),
        None => Err(ResponseError::NotFound(Section::Interval)),
    }
}

fn init_complete(dic_response: &DicValues) -> ResultResponse<u64> {
    match dic_response.get(&COMPLETE.as_bytes().to_vec()) {
        Some(ValuesBencoding::Integer(complete)) => Ok(*complete as u64),
        Some(_) => Err(ResponseError::Format(Section::Complete)),
        None => Err(ResponseError::NotFound(Section::Complete)),
    }
}

fn init_incomplete(dic_response: &DicValues) -> ResultResponse<u64> {
    match dic_response.get(&INCOMPLETE.as_bytes().to_vec()) {
        Some(ValuesBencoding::Integer(incomplete)) => Ok(*incomplete as u64),
        Some(_) => Err(ResponseError::Format(Section::Incomplete)),
        None => Err(ResponseError::NotFound(Section::Incomplete)),
    }
}

fn get_peer_id(dic_peer: &DicValues) -> ResultResponse<Vec<u8>> {
    match dic_peer.get(&PEER_ID.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(peer_id)) => Ok(peer_id.clone()),
        Some(_) => Err(ResponseError::Format(Section::PeerId)),
        None => Err(ResponseError::NotFound(Section::PeerId)),
    }
}

fn get_ip(dic_peer: &DicValues) -> ResultResponse<String> {
    match dic_peer.get(&IP.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(ip)) => Ok(String::from_utf8_lossy(ip).to_string()),
        Some(_) => Err(ResponseError::Format(Section::Ip)),
        None => Err(ResponseError::NotFound(Section::Ip)),
    }
}

fn get_port(dic_peer: &DicValues) -> ResultResponse<u16> {
    match dic_peer.get(&PORT.as_bytes().to_vec()) {
        Some(ValuesBencoding::Integer(port)) => Ok(*port as u16),
        Some(_) => Err(ResponseError::Format(Section::Port)),
        None => Err(ResponseError::NotFound(Section::Port)),
    }
}

fn init_peers(dic_response: &DicValues) -> ResultResponse<Vec<PeerDataFromTrackerResponse>> {
    let mut vector_peers = vec![];

    match dic_response.get(&PEERS.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list_peers)) => {
            for peer in list_peers {
                match peer {
                    ValuesBencoding::Dic(dic_peer) => {
                        let ip = get_ip(dic_peer)?;
                        let port = get_port(dic_peer)?;
                        let peer_id = get_peer_id(dic_peer)?;
                        let peer_struct = PeerDataFromTrackerResponse::new(peer_id, ip, port)?;
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
        let interval = init_interval(&dic_response)?;
        let complete = init_complete(&dic_response)?;
        let incomplete = init_incomplete(&dic_response)?;
        let peers = init_peers(&dic_response)?;

        Ok(TrackerResponseData {
            interval,
            complete,
            incomplete,
            peers,
        })
    }
}
