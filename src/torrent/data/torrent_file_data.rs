//! # Modulo de data de un archivo .torrent
//! Este modulo contiene las funciones encargadas de almacenar solo la info
//! importante a partir de la info completa de un .torrent interpretado.

use sha1::{Digest, Sha1};
use std::{collections::HashMap, error::Error, fmt};

use crate::torrent::parsers::{bencoding::values::ValuesBencoding, *};
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
///Enumerado que representa la seccion en la que el error puede surgir
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
///Enumerado que representa el tipo de error que puede surgir, que por dentreo tendra
/// su seccion correspondiente
pub enum TorrentError {
    NotFound(Section),
    Format(Section),
}

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

fn init_size_files(dic_info: &DicValues) -> Result<Vec<i64>, TorrentError> {
    let mut vec_sizes = vec![];
    //Si es single file solo tomo el valor de length
    if let Some(ValuesBencoding::Integer(length)) = dic_info.get(&LENGTH.as_bytes().to_vec()) {
        vec_sizes.push(*length);
        return Ok(vec_sizes);
    }
    //Si es multiple file recorro la lista de diccionarios de todos los archivos y tomo sus lengths
    match dic_info.get(&FILES.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list_files)) => {
            for file in list_files {
                if let ValuesBencoding::Dic(dic_file) = file {
                    match dic_file.get(&LENGTH.as_bytes().to_vec()) {
                        Some(ValuesBencoding::Integer(length)) => vec_sizes.push(*length),
                        Some(_) => return Err(TorrentError::Format(Section::Length)),
                        None => return Err(TorrentError::NotFound(Section::Length)),
                    }
                }
            }
            Ok(vec_sizes)
        }
        Some(_) => Err(TorrentError::Format(Section::Files)),
        None => Err(TorrentError::NotFound(Section::Files)),
    }
}

fn init_total_size(vec_size: Vec<i64>) -> i64 {
    let mut total_size = 0;

    for size in vec_size {
        total_size += size;
    }
    total_size
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
    ///Funcion para crear un TorrentFileData, necesita que se le pase un HashMap que tenga Vec<u8> como clave
    /// y ValuesBencoding como valores con los campos requeridos de un archivo .torrent, en caso de que no
    /// contenga alguno o haya formatos distintos a los deseados se devolvera el error correspondiente
    ///
    pub fn new(dic_torrent: DicValues) -> Result<Self, TorrentError> {
        let info = init_info(&dic_torrent)?;

        let piece_length = init_piece_length(&info)?;
        let size_files = init_size_files(&info)?;
        let total_size = init_total_size(size_files);
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

    ///Funcion que devuelve true si el Torrent representado es Single File y false si es Multiple File
    ///
    pub fn is_single_file(&self) -> bool {
        self.is_single_file
    }

    ///Funcion que devuelve la url del tracker principal del Torrent
    ///
    pub fn get_tracker_main(&self) -> String {
        self.url_tracker_main.clone()
    }

    ///Funcion que devuelve el campo name del Torrent.
    ///
    /// Este valor varia dependiendo del tipo de Torrent que es, en caso de ser single file, este sera
    /// el nombre del archivo y en caso de ser multiple file este sera el nombre de la carpeta contenedora
    /// de los archivos.
    ///
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    ///Funcion que va a devolver el info_hash, que es el campo info del .torrent bencodeado y encriptado mediante
    /// SHA-1
    ///
    pub fn get_info_hash(&self) -> Vec<u8> {
        self.info_hash.clone()
    }

    ///Funcion que devuelve el tamaño total de todos los archivos
    ///
    pub fn get_total_size(&self) -> i64 {
        self.total_size
    }

    ///Funcion que devuelve el largo que van a tener las piezas
    ///
    pub fn get_piece_length(&self) -> i64 {
        self.piece_length
    }

    ///Funcion que devuelve la cantidad total de piezas que va a tener el archivo
    ///
    pub fn get_total_amount_pieces(&self) -> usize {
        self.total_amount_pieces
    }

    ///Funcion que va a devolver los path de los archivos [Solo se utiliza en caso de que el torrent
    /// sea multiple file]
    ///
    pub fn get_paths(&self) -> Vec<String> {
        self.path.clone()
    }

    ///Funcion que dado el numero de pieza devuelve su encriptacion en SHA-1
    ///
    pub fn get_piece_sha1(&self, piece_index: usize) -> Vec<u8> {
        let mut pieces_return = vec![];
        let mut long_sha1 = 20;
        let mut iterator = self.pieces.clone().into_iter();
        if piece_index >= self.get_total_amount_pieces() {
            return pieces_return;
        } else if piece_index > 0 {
            let init_pos = (piece_index * long_sha1) - 1;
            iterator.nth(init_pos);
        }
        while long_sha1 > 0 {
            match iterator.next() {
                Some(byte) => pieces_return.push(byte),
                None => return pieces_return,
            };
            long_sha1 -= 1;
        }
        pieces_return
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

        let name_expected = String::from("ubuntu-14.04.6-server-ppc64el.iso");
        let pieces_length_expected = 524288;
        let total_size_expected = 600401920;
        let total_pieces_expected = 1146;

        assert_eq!(torrent.get_name(), name_expected);
        assert_eq!(torrent.get_tracker_main(), tracker_main);
        assert_eq!(torrent.get_piece_length(), pieces_length_expected);
        assert_eq!(torrent.get_total_size(), total_size_expected);
        assert_eq!(torrent.get_total_amount_pieces(), total_pieces_expected);

        let first_id = 0;
        let last_id = torrent.get_total_amount_pieces() - 1;

        let first_piece = torrent.get_piece_sha1(first_id);
        let last_piece = torrent.get_piece_sha1(last_id);

        let long_sha1 = 20;
        let pos_last_piece = torrent.pieces.len() - long_sha1;

        assert_eq!(first_piece, torrent.pieces[..long_sha1]);
        assert_eq!(last_piece, torrent.pieces[pos_last_piece..]);
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
        let name_expected = String::from("Big Buck Bunny");
        let pieces_length_expected = 262144;
        let total_size_expected = 140 + 276134947 + 310380;
        let total_pieces_expected = 1055;

        assert_eq!(torrent.get_name(), name_expected);
        assert_eq!(torrent.get_tracker_main(), tracker_main);
        assert_eq!(torrent.get_piece_length(), pieces_length_expected);
        assert_eq!(torrent.get_total_size(), total_size_expected);
        assert_eq!(torrent.get_total_amount_pieces(), total_pieces_expected);
        assert_eq!(torrent.get_paths(), paths_expected);

        let first_id = 0;
        let last_id = torrent.get_total_amount_pieces() - 1;

        let first_piece = torrent.get_piece_sha1(first_id);
        let last_piece = torrent.get_piece_sha1(last_id);

        let long_sha1 = 20;
        let pos_last_piece = torrent.pieces.len() - long_sha1;

        assert_eq!(first_piece, torrent.pieces[..long_sha1]);
        assert_eq!(last_piece, torrent.pieces[pos_last_piece..]);
    }
}
