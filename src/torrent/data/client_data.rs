use crate::torrent::parsers::p2p::message::PieceStatus;

#[derive(PartialEq, Debug, Clone)]
pub enum ClientState {
    Started,
    Completed,
    Stopped,
}

#[derive(PartialEq, Debug, Clone)]
pub struct ClientData {
    pub peer_id: String,
    pub info_hash: Vec<u8>,
    //pub address: SocketAddr,
    pub uploaded: u32,
    pub downloaded: u32,
    pub left: u32,
    pub event: ClientState,
    pub pieces_availability: Vec<PieceStatus>,
    //..
}
