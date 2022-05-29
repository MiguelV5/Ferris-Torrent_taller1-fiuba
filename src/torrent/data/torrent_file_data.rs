#![allow(dead_code)]
use sha1::{Digest, Sha1};
use std::{collections::HashMap, error::Error, fmt};

use super::super::parsers::bencoding::values::ValuesBencoding;
use crate::torrent::parsers::*;

type DicValues = HashMap<Vec<u8>, ValuesBencoding>;

const ANNOUNCE: &str = "announce";
const INFO: &str = "info";
const PIECE_LENGTH: &str = "piece length";
const ANNOUNCE_LIST: &str = "announce-list";
const LENGHT: &str = "length";
const FILES: &str = "files";

#[derive(Debug, PartialEq)]
pub enum Section {
    Announce,
    Info,
    PieceLength,
    Files,
    Length,
}

#[derive(Debug, PartialEq)]
pub enum ErrorTorrent {
    NotFound(Section),
    Format(Section),
}
//Queda todavia por hacer:
// *Cambio de tracker_main por si el primero no funciona
// *Ver como manejarse con el name y path dependiendo de si es single file o multiple file
// *Con multiple file ver como guardar el path de cada archivo o como devolverlo en una funcion quizas
#[derive(PartialEq, Debug, Clone)]
pub struct TorrentFileData {
    pub url_tracker_main: String,
    pub url_tracker_list: Vec<ValuesBencoding>,
    pub info: DicValues,
    pub info_hash: Vec<u8>,
    pub piece_length: i64,
    pub total_amount_pieces: usize,
    pub total_size: i64,
}

impl fmt::Display for ErrorTorrent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error del torrent.\n Backtrace: {:?}\n", self)
    }
}

impl Error for ErrorTorrent {}

fn vec_u8_to_string(vec: &[u8]) -> String {
    String::from_utf8_lossy(vec).to_string()
}

fn init_tracker_main(dic_torrent: &DicValues) -> Result<String, ErrorTorrent> {
    match dic_torrent.get(&ANNOUNCE.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(tracker)) => Ok(vec_u8_to_string(tracker)),
        Some(_) => Err(ErrorTorrent::Format(Section::Announce)),
        None => Err(ErrorTorrent::NotFound(Section::Announce)),
    }
}

fn init_info(dic_torrent: &DicValues) -> Result<DicValues, ErrorTorrent> {
    match dic_torrent.get(&INFO.as_bytes().to_vec()) {
        Some(ValuesBencoding::Dic(dic_info)) => Ok(dic_info.clone()),
        Some(_) => Err(ErrorTorrent::Format(Section::Info)),
        None => Err(ErrorTorrent::NotFound(Section::Info)),
    }
}

fn init_info_hash(dic_info: &DicValues) -> Result<Vec<u8>, ErrorTorrent> {
    //Paso info a bencoding
    let vec_info = bencoding::encoder::from_dic(dic_info.clone());
    //Le aplico SHA-1 a el info bencodeado
    let mut hasher = Sha1::new();
    hasher.update(vec_info);
    let result = hasher.finalize();
    let vec_sha1 = result.as_slice().to_vec();
    //Ahora le aplico url encoding
    let info_hash = url_encoder::from_string(vec_sha1);

    Ok(info_hash)
}

fn init_piece_length(dic_info: &DicValues) -> Result<i64, ErrorTorrent> {
    match dic_info.get(&PIECE_LENGTH.as_bytes().to_vec()) {
        Some(ValuesBencoding::Integer(length)) => Ok(*length),
        Some(_) => Err(ErrorTorrent::Format(Section::PieceLength)),
        None => Err(ErrorTorrent::NotFound(Section::PieceLength)),
    }
}

fn init_tracker_list(dic_torrent: &DicValues) -> Result<Vec<ValuesBencoding>, ErrorTorrent> {
    match dic_torrent.get(&ANNOUNCE_LIST.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list)) => Ok(list.clone()),
        _ => Ok(vec![]),
    }
}

fn init_total_size(dic_info: &DicValues) -> Result<i64, ErrorTorrent> {
    //Si es single file solo tomo el valor de length
    if let Some(ValuesBencoding::Integer(length)) = dic_info.get(&LENGHT.as_bytes().to_vec()) {
        return Ok(*length);
    }
    //Si es multiple file recorro la lista de diccionarios de todos los archivos y sumo sus lengths
    let mut length_total = 0;
    match dic_info.get(&FILES.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list_files)) => {
            for file in list_files {
                if let ValuesBencoding::Dic(dic_file) = file {
                    match dic_file.get(&LENGHT.as_bytes().to_vec()) {
                        Some(ValuesBencoding::Integer(length)) => length_total += length,
                        Some(_) => return Err(ErrorTorrent::Format(Section::Length)),
                        None => return Err(ErrorTorrent::NotFound(Section::Length)),
                    }
                }
            }
            Ok(length_total)
        }
        Some(_) => Err(ErrorTorrent::Format(Section::Files)),
        None => Err(ErrorTorrent::NotFound(Section::Files)),
    }
}

impl TorrentFileData {
    pub fn new(dic_torrent: DicValues) -> Result<Self, ErrorTorrent> {
        let mut torrent = TorrentFileData {
            url_tracker_main: init_tracker_main(&dic_torrent)?,
            url_tracker_list: init_tracker_list(&dic_torrent)?,
            info: init_info(&dic_torrent)?,
            info_hash: vec![],
            total_size: 0,
            piece_length: 0,
            total_amount_pieces: 0,
        };
        torrent.info_hash = init_info_hash(&torrent.info)?;
        torrent.piece_length = init_piece_length(&torrent.info)?;
        torrent.total_size = init_total_size(&torrent.info)?;
        let mut total_amount_pieces =
            (torrent.get_total_size() / torrent.get_piece_length()) as usize;
        if torrent.total_size % torrent.piece_length > 0 {
            total_amount_pieces += 1;
        }
        torrent.total_amount_pieces = total_amount_pieces;

        Ok(torrent)
    }

    pub fn get_tracker_main(&self) -> String {
        self.url_tracker_main.clone()
    }

    pub fn get_info_hash(&self) -> Vec<u8> {
        self.info_hash.clone()
    }

    pub fn get_total_size(&self) -> i64 {
        self.total_size
    }

    pub fn get_piece_length(&self) -> i64 {
        self.piece_length
    }

    pub fn get_total_amount_pieces(&self) -> usize {
        self.total_amount_pieces
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::torrent::data::medatada_analyzer::read_torrent_file_to_dic;

    #[test]
    fn test_torrent_single_file_ok() {
        //ubuntu-14.04.6-server-ppc64el.iso [un solo archivo y un solo tracker]
        let dir = "torrents_for_test/ubuntu-14.04.6-server-ppc64el.iso.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => panic!("{:?}", error),
        };

        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => panic!("{:?}", error),
        };

        let tracker_main = String::from("http://torrent.ubuntu.com:6969/announce");

        assert_eq!(torrent.get_tracker_main(), tracker_main);
        assert_eq!(torrent.get_piece_length(), 524288);
        assert_eq!(torrent.get_total_size(), 600401920);
        assert_eq!(torrent.get_total_amount_pieces(), 1146);
    }
    #[test]
    fn test_torrent_multiple_file_ok() {
        let dir = "torrents_for_test/big-buck-bunny.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => panic!("{:?}", error),
        };

        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => panic!("{:?}", error),
        };

        let tracker_main = String::from("udp://tracker.leechers-paradise.org:6969");

        assert_eq!(torrent.get_tracker_main(), tracker_main);
        assert_eq!(torrent.get_piece_length(), 262144);
        assert_eq!(torrent.get_total_size(), 140 + 276134947 + 310380);
        assert_eq!(torrent.get_total_amount_pieces(), 1055);
    }
}
