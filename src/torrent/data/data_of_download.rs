#![allow(dead_code)]
use crate::torrent::parsers::p2p::message::PieceStatus;

#[derive(PartialEq, Debug, Clone)]
pub enum StateOfDownload {
    Started,
    Completed,
    Stopped,
}

#[derive(PartialEq, Debug, Clone)]
pub struct DataOfDownload {
    //pub address: SocketAddr,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: StateOfDownload,
    pub pieces_availability: Vec<PieceStatus>,
    //..
}

impl DataOfDownload {
    pub fn new(size_torrent: u64) -> Self {
        DataOfDownload {
            uploaded: 0,
            downloaded: 0,
            left: size_torrent,
            event: StateOfDownload::Started,
            pieces_availability: vec![],
        }
    }
}
