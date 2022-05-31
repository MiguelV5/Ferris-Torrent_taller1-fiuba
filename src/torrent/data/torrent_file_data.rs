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
const LENGTH: &str = "length";
const FILES: &str = "files";
const NAME: &str = "name";
const PIECES: &str = "pieces";
const PATH: &str = "path";

#[derive(Debug, PartialEq)]
pub enum Section {
    Announce,
    Info,
    PieceLength,
    Files,
    Length,
    Name,
    Pieces,
    Path,
}

#[derive(Debug, PartialEq)]
pub enum TorrentError {
    NotFound(Section),
    Format(Section),
}
//Queda todavia por hacer:
// *Cambio de tracker_main por si el primero no funciona
// *Ver como manejarse con el name y path dependiendo de si es single file o multiple file
// *Con multiple file ver como guardar el path de cada archivo o como devolverlo en una funcion quizas
#[derive(PartialEq, Debug, Clone)]
pub struct TorrentFileData {
    pub is_single_file: bool,
    pub url_tracker_main: String,
    pub url_tracker_list: Vec<ValuesBencoding>,
    pub name: String,
    pub pieces: Vec<u8>,
    pub info_hash: Vec<u8>,
    pub piece_length: i64,
    pub total_amount_pieces: usize,
    pub total_size: i64,
    pub path: Vec<String>,
}

impl fmt::Display for TorrentError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error de TorrentFileData.\n Backtrace: {:?}\n", self)
    }
}

impl Error for TorrentError {}

fn vec_u8_to_string(vec: &[u8]) -> String {
    String::from_utf8_lossy(vec).to_string()
}

fn init_name(dic_torrent: &DicValues) -> Result<String, TorrentError> {
    match dic_torrent.get(&NAME.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(name)) => Ok(vec_u8_to_string(name)),
        Some(_) => Err(TorrentError::Format(Section::Name)),
        None => Err(TorrentError::NotFound(Section::Name)),
    }
}

fn verify_single_file(dic_info: &DicValues) -> bool {
    dic_info.get(&FILES.as_bytes().to_vec()).is_none()
}

fn init_tracker_main(dic_torrent: &DicValues) -> Result<String, TorrentError> {
    match dic_torrent.get(&ANNOUNCE.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(tracker)) => Ok(vec_u8_to_string(tracker)),
        Some(_) => Err(TorrentError::Format(Section::Announce)),
        None => Err(TorrentError::NotFound(Section::Announce)),
    }
}

fn init_info(dic_torrent: &DicValues) -> Result<DicValues, TorrentError> {
    match dic_torrent.get(&INFO.as_bytes().to_vec()) {
        Some(ValuesBencoding::Dic(dic_info)) => Ok(dic_info.clone()),
        Some(_) => Err(TorrentError::Format(Section::Info)),
        None => Err(TorrentError::NotFound(Section::Info)),
    }
}

fn init_info_hash(dic_info: &DicValues) -> Result<Vec<u8>, TorrentError> {
    //Paso info a bencoding
    let vec_info = bencoding::encoder::from_dic(dic_info.clone());
    //Le aplico SHA-1 a el info bencodeado
    let mut hasher = Sha1::new();
    hasher.update(vec_info);
    let result = hasher.finalize();
    let vec_sha1 = result.as_slice().to_vec();

    Ok(vec_sha1)
}

fn init_piece_length(dic_info: &DicValues) -> Result<i64, TorrentError> {
    match dic_info.get(&PIECE_LENGTH.as_bytes().to_vec()) {
        Some(ValuesBencoding::Integer(length)) => Ok(*length),
        Some(_) => Err(TorrentError::Format(Section::PieceLength)),
        None => Err(TorrentError::NotFound(Section::PieceLength)),
    }
}

fn init_tracker_list(dic_torrent: &DicValues) -> Result<Vec<ValuesBencoding>, TorrentError> {
    match dic_torrent.get(&ANNOUNCE_LIST.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list)) => Ok(list.clone()),
        _ => Ok(vec![]),
    }
}

fn init_total_size(dic_info: &DicValues) -> Result<i64, TorrentError> {
    //Si es single file solo tomo el valor de length
    if let Some(ValuesBencoding::Integer(length)) = dic_info.get(&LENGTH.as_bytes().to_vec()) {
        return Ok(*length);
    }
    //Si es multiple file recorro la lista de diccionarios de todos los archivos y sumo sus lengths
    let mut length_total = 0;
    match dic_info.get(&FILES.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list_files)) => {
            for file in list_files {
                if let ValuesBencoding::Dic(dic_file) = file {
                    match dic_file.get(&LENGTH.as_bytes().to_vec()) {
                        Some(ValuesBencoding::Integer(length)) => length_total += length,
                        Some(_) => return Err(TorrentError::Format(Section::Length)),
                        None => return Err(TorrentError::NotFound(Section::Length)),
                    }
                }
            }
            Ok(length_total)
        }
        Some(_) => Err(TorrentError::Format(Section::Files)),
        None => Err(TorrentError::NotFound(Section::Files)),
    }
}

fn init_total_amount_pieces(total_size: i64, piece_lenght: i64) -> usize {
    let mut total_amount_pieces = (total_size / piece_lenght) as usize;
    if total_size % piece_lenght > 0 {
        total_amount_pieces += 1;
    }
    total_amount_pieces
}

fn init_pieces(dic_info: &DicValues) -> Result<Vec<u8>, TorrentError> {
    match dic_info.get(&PIECES.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(pieces)) => Ok(pieces.clone()),
        Some(_) => Err(TorrentError::Format(Section::Pieces)),
        None => Err(TorrentError::NotFound(Section::Pieces)),
    }
}

fn string_of_path(path: Vec<ValuesBencoding>) -> Result<String, TorrentError> {
    let mut return_str = String::new();
    for value in path {
        return_str.push('/');
        match value {
            ValuesBencoding::String(dir) => return_str.push_str(&vec_u8_to_string(&dir)),
            _ => return Err(TorrentError::Format(Section::Path)),
        }
    }
    Ok(return_str)
}

fn init_path(dic_info: &DicValues) -> Result<Vec<String>, TorrentError> {
    let mut return_vec = vec![];
    match dic_info.get(&FILES.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list_files)) => {
            for file in list_files {
                if let ValuesBencoding::Dic(dic_file) = file {
                    match dic_file.get(&PATH.as_bytes().to_vec()) {
                        Some(ValuesBencoding::List(path)) => {
                            let str_path = string_of_path(path.clone())?;
                            return_vec.push(str_path);
                        }
                        Some(_) => return Err(TorrentError::Format(Section::Length)),
                        None => return Err(TorrentError::NotFound(Section::Length)),
                    }
                }
            }
            Ok(return_vec)
        }
        Some(_) => Err(TorrentError::Format(Section::Files)),
        None => Ok(vec![]),
    }
}

impl TorrentFileData {
    pub fn new(dic_torrent: DicValues) -> Result<Self, TorrentError> {
        let info = init_info(&dic_torrent)?;

        let piece_length = init_piece_length(&info)?;
        let total_size = init_total_size(&info)?;
        let total_amount_pieces = init_total_amount_pieces(total_size, piece_length);

        Ok(TorrentFileData {
            is_single_file: verify_single_file(&dic_torrent),
            url_tracker_main: init_tracker_main(&dic_torrent)?,
            url_tracker_list: init_tracker_list(&dic_torrent)?,
            name: init_name(&info)?,
            info_hash: init_info_hash(&info)?,
            pieces: init_pieces(&info)?,
            path: init_path(&info)?,
            total_size,
            piece_length,
            total_amount_pieces,
        })
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

    pub fn get_paths(&self) -> Vec<String> {
        self.path.clone()
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
        let first_path = String::from("/Big Buck Bunny.en.srt");
        let second_path = String::from("/Big Buck Bunny.mp4");
        let third_path = String::from("/poster.jpg");
        let paths_expected = vec![first_path, second_path, third_path];

        assert_eq!(torrent.get_tracker_main(), tracker_main);
        assert_eq!(torrent.get_piece_length(), 262144);
        assert_eq!(torrent.get_total_size(), 140 + 276134947 + 310380);
        assert_eq!(torrent.get_total_amount_pieces(), 1055);
        assert_eq!(torrent.get_paths(), paths_expected)
    }
}
