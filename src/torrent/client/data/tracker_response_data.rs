#![allow(dead_code)]

use std::net::SocketAddr;

pub struct TrackerResponsePeerData {
    pub peer_id: Option<String>,
    pub peer_address: SocketAddr,
}

pub struct TrackerResponseData {
    pub interval: u32,
    pub tracker_id: String,
    pub complete: u32,
    pub incomplete: u32,
    pub peers: Vec<TrackerResponsePeerData>,
}
