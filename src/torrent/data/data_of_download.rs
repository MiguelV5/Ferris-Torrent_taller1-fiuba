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
    pub uploaded: u32,
    pub downloaded: u32,
    pub left: u32,
    pub event: StateOfDownload,
    pub pieces_availability: Vec<PieceStatus>,
    //..
}
