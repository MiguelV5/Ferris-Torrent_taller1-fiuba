#![allow(dead_code)]
extern crate rand;

use crate::torrent::data::torrent_file_data::TorrentFileData;
use crate::torrent::{
    client::tracker_comunication::http_handler::HttpHandler,
    parsers::bencoding::values::ValuesBencoding,
};
use rand::{distributions::Alphanumeric, *};
use std::{collections::HashMap, error::Error};

const SIZE_PEER_ID: usize = 20;

pub fn init_communication(
    torrent: TorrentFileData,
) -> Result<HashMap<Vec<u8>, ValuesBencoding>, Box<dyn Error>> {
    //Solo lo puse todo entero aca para que quede el ejemplo de como crear el dicc de la respuesta por ahora
    let http_handler = HttpHandler::new(torrent, generate_peer_id())?;
    let response = http_handler.tracker_get_response()?;
    //println!("{:?}", response);
    Ok(response)
}

pub fn generate_peer_id() -> String {
    let peer_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(SIZE_PEER_ID)
        .map(char::from)
        .collect();
    peer_id
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::torrent::data::medatada_analyzer::read_torrent_file_to_dic;
    #[test]
    fn test_generate_peer_id_ok() {
        let peer_id = generate_peer_id();
        assert!(peer_id.len() == 20);
        println!("{:?}", peer_id);
    }
    #[test]
    fn test_connect() {
        let dir = "torrents_for_test/ubuntu-22.04-desktop-amd64.iso.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => panic!("{:?}", error),
        };

        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => panic!("{:?}", error),
        };

        assert_eq!(torrent.get_piece_lenght(), 262144) //Placeholder para que no de warnings

        //init_communication(torrent); //descomentar para ver como devuelve el HashMap de la respuesta
        //Habria que ver como testear la comunicacion ya que podria dar bien, como mal, ya que puede que el tracker
        //este ocupado o haya algun otro tipo de inconveniente, coneccion a internet, etc.
    }
}
