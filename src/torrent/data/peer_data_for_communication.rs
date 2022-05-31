use crate::torrent::parsers::p2p::message::PieceStatus;

#[derive(PartialEq, Debug, Clone)]
pub struct PeerDataForP2PCommunication {
    pub peer_id: Vec<u8>,
    pub pieces_availability: Option<Vec<PieceStatus>>,
    pub am_choking: bool,
    pub am_interested: bool,
    pub peer_choking: bool,
    //pub peer_interested: bool, //por ahora no me interesa
    //...
}

// #[derive(PartialEq, Debug, Clone)]
// pub struct PeersDataList {
//     pub total_amount_of_peers: u32,
//     pub data_list: Vec<PeerDataForP2PCommunication>,
// }
// Creo que no es necesario ya que es simplemente un vec con su largo. Recordar que en Rust los Vec tienen la ventaja de calcular su .len() como O(1) porque es un campo de la abstraccion len. Me quede sorprendido cuando lo vi
