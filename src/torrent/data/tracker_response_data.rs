#![allow(dead_code)]

use std::net::SocketAddr;

#[derive(PartialEq, Debug, Clone)]
pub struct PeerDataFromTrackerResponse {
    pub peer_id: Option<String>,
    pub peer_address: SocketAddr,
}

#[derive(PartialEq, Debug, Clone)]

pub struct TrackerResponseData {
    pub interval: u32,
    pub tracker_id: String,
    pub complete: u32,
    pub incomplete: u32,
    pub peers: Vec<PeerDataFromTrackerResponse>,
}
