use crate::torrent::parsers::p2p::message::PieceStatus;

#[derive(PartialEq, Debug, Clone)]
pub struct PeerData {
    pub peer_id: String,
    pub pieces_availability: Option<Vec<PieceStatus>>,
    pub am_chocking: bool,
    pub am_interested: bool,
    pub peer_choking: bool,
    //pub peer_interested: bool, //por ahora no me interesa
    //...
}

#[derive(PartialEq, Debug, Clone)]
pub struct PeersDataList {
    pub total_amount_of_peers: u32,
    pub data_list: Vec<PeerData>,
}
