#![allow(dead_code)]
use crate::torrent::data::medatada_analyzer::MetadataError;
use crate::torrent::data::torrent_file_data::TorrentError;
use crate::torrent::data::tracker_response_data::ResponseError;
use crate::torrent::data::{
    data_of_download::DataOfDownload, peer_data_for_communication::PeerDataForP2PCommunication,
    torrent_file_data::TorrentFileData, tracker_response_data::TrackerResponseData,
};

extern crate rand;

use crate::torrent::client::tracker_comunication::http_handler::HttpHandler;
use crate::torrent::data::medatada_analyzer::read_torrent_file_to_dic;
use crate::torrent::parsers::bencoding::values::ValuesBencoding;
use log::{debug, error, trace};
use rand::{distributions::Alphanumeric, Rng};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use super::tracker_comunication::http_handler::ErrorMsgHttp;
const SIZE_PEER_ID: usize = 12;
const INIT_PEER_ID: &str = "-FA0000-";

type DicValues = HashMap<Vec<u8>, ValuesBencoding>;
type ResultClient<T> = Result<T, ClientError>;

#[derive(PartialEq, Debug, Clone)]
pub struct Client {
    pub peer_id: Vec<u8>,
    pub info_hash: Vec<u8>,

    pub data_of_download: DataOfDownload,
    pub torrent_file: TorrentFileData,
    pub tracker_response: Option<TrackerResponseData>,
    pub list_of_peers_data_for_communication: Option<Vec<PeerDataForP2PCommunication>>,
}

#[derive(PartialEq, Debug)]
pub enum ClientError {
    File(MetadataError),
    HttpCreation(ErrorMsgHttp),
    ConectionError(ErrorMsgHttp),
    TorrentCreation(TorrentError),
    Response(ResponseError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error del torrent.\n Backtrace: {:?}\n", self)
    }
}

impl Error for ClientError {}

pub fn generate_peer_id() -> Vec<u8> {
    let rand_alphanumeric: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(SIZE_PEER_ID)
        .map(char::from)
        .collect();
    let str_peer = format!("{}{}", INIT_PEER_ID, rand_alphanumeric);
    debug!("Peer_id: {}", str_peer);
    str_peer.as_bytes().to_vec()
}

pub fn create_torrent(torrent_path: &str) -> ResultClient<TorrentFileData> {
    trace!("Leyendo el archivo para poder crear el torrent");
    let torrent_dic = match read_torrent_file_to_dic(torrent_path) {
        Ok(dictionary) => dictionary,
        Err(error) => {
            error!("Error del cliente al leer archivo y pasarlo a HashMap");
            return Err(ClientError::File(error));
        }
    };
    trace!("Arhivo leido y pasado a HashMap exitosamente");
    trace!("Creando TorrentFileData");
    match TorrentFileData::new(torrent_dic) {
        Ok(torrent) => Ok(torrent),
        Err(error) => {
            error!("Error del cliente al crear la estructura del torrent");
            Err(ClientError::TorrentCreation(error))
        }
    }
}

pub fn init_communication(torrent: TorrentFileData) -> ResultClient<TrackerResponseData> {
    let str_peer_id = String::from_utf8_lossy(&generate_peer_id()).to_string();
    trace!("Creando httpHandler dentro del Client");
    let http_handler = match HttpHandler::new(torrent, str_peer_id) {
        Ok(http) => http,
        Err(error) => {
            error!("Error del cliente al crear HttpHandler");
            return Err(ClientError::HttpCreation(error));
        }
    };
    trace!("HttpHandler creado exitosamente");
    trace!("Comunicacion con el Tracker mediante httpHandler");
    let response_tracker = match http_handler.tracker_get_response() {
        Ok(response) => response,
        Err(error) => {
            return {
                error!("Error del cliente al conectarse con el Tracker");
                Err(ClientError::ConectionError(error))
            }
        }
    };
    trace!("Creando el TrackerResponseData en base a la respues del tracker");
    match TrackerResponseData::new(response_tracker) {
        Ok(response_struct) => Ok(response_struct),
        Err(error) => {
            error!("Error del cliente al recibir respuesta del Tracker");
            Err(ClientError::Response(error))
        }
    }
}

impl Client {
    pub fn new(path_file: &str) -> ResultClient<Self> {
        trace!("Genero peer_id");
        let peer_id = generate_peer_id();
        let torrent_file = create_torrent(path_file)?;
        trace!("TorrentFileData creado y almacenado dentro del Client");
        let info_hash = torrent_file.get_info_hash();
        let torrent_size = torrent_file.get_total_size() as u64;
        let data_of_download = DataOfDownload::new(torrent_size);
        Ok(Client {
            peer_id,
            torrent_file,
            data_of_download,
            info_hash,
            tracker_response: None,
            list_of_peers_data_for_communication: None,
        })
    }
    pub fn init_communication(&mut self) -> ResultClient<()> {
        match init_communication(self.torrent_file.clone()) {
            Ok(response) => self.tracker_response = Some(response),
            Err(error) => return Err(error),
        };
        trace!("TrackerResponseData creado y almacenado dentro del Client");
        Ok(())
    }
}
