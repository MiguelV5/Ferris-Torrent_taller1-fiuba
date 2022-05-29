#![allow(dead_code)]
use crate::torrent::data::{
    data_of_download::DataOfDownload, peer_data_for_communication::PeerDataForP2PCommunication,
    torrent_file_data::TorrentFileData, tracker_response_data::TrackerResponseData,
};

#[derive(PartialEq, Debug, Clone)]
pub struct Client {
    pub peer_id: String,
    pub info_hash: Vec<u8>,

    pub data_of_download: DataOfDownload,
    pub torrent_file: TorrentFileData,
    pub tracker_response: TrackerResponseData, //deberia ser un Option<>
    pub list_of_peers_data_for_communication: Option<Vec<PeerDataForP2PCommunication>>,
}

// impl Client {
//     se podria implementar la creacin de un cliente a partir de un bloqe de datos dado o algo por el estilo
//     fn new() -> Self {}
// }
