//! # Modulo de data de un archivo .torrent
//! Este modulo contiene las funciones encargadas de almacenar solo la info
//! importante a partir de la info completa de un .torrent interpretado.

use sha1::{Digest, Sha1};
use std::ffi::OsStr;
use std::path::Path;
use std::{collections::HashMap, error::Error, fmt};

use crate::torrent::client::peers_communication::handler_communication::BLOCK_BYTES;
use crate::torrent::parsers::p2p::message::PieceStatus;
use crate::torrent::parsers::{bencoding::values::ValuesBencoding, *};
use rand::seq::SliceRandom;
use rand::thread_rng;
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
    TrackerList,
    Info,
    PieceLength,
    Files,
    Length,
    Name,
    Pieces,
    Path,
    FilesData,
}

#[derive(Debug, PartialEq)]
///Enumerado que representa el tipo de error que puede surgir, que por dentreo tendra
/// su seccion correspondiente
pub enum TorrentFileDataError {
    NotFound(Section),
    Format(Section),
    Creation(Section),
    Calculation(Section),
    CheckingBitfield(String),
    CheckingRequestBlock(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct FileData {
    pub path: String,
    pub file_length: u64,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TargetFilesData {
    SingleFile {
        file_name: String,
        file_length: u64,
    },
    MultipleFiles {
        dir_name: String,
        list_of_files_data: Vec<FileData>,
    },
}

#[derive(PartialEq, Debug, Clone)]
pub struct TorrentFileData {
    pub url_tracker_main: String,
    pub url_tracker_list: Vec<String>,
    pub sha1_pieces: Vec<u8>,
    pub sha1_info_hash: Vec<u8>,
    pub piece_length: u64,
    pub total_length: u64,
    pub total_amount_of_pieces: usize,
    pub target_files_data: TargetFilesData,
}

impl fmt::Display for TorrentFileDataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for TorrentFileDataError {}

fn vec_u8_to_string(vec: &[u8]) -> String {
    String::from_utf8_lossy(vec).to_string()
}
fn init_name(dic_torrent: &DicValues) -> Result<String, TorrentFileDataError> {
    match dic_torrent.get(&NAME.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(name)) => Ok(vec_u8_to_string(name)),
        Some(_) => Err(TorrentFileDataError::Format(Section::Name)),
        None => Err(TorrentFileDataError::NotFound(Section::Name)),
    }
}

fn is_single_file(dic_info: &DicValues) -> bool {
    dic_info.get(&FILES.as_bytes().to_vec()).is_none()
}

fn init_tracker_main(dic_torrent: &DicValues) -> Result<String, TorrentFileDataError> {
    match dic_torrent.get(&ANNOUNCE.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(tracker)) => Ok(vec_u8_to_string(tracker)),
        Some(_) => Err(TorrentFileDataError::Format(Section::Announce)),
        None => Err(TorrentFileDataError::NotFound(Section::Announce)),
    }
}

fn init_tracker_list(dic_torrent: &DicValues) -> Result<Vec<String>, TorrentFileDataError> {
    let mut list_tracker_url = vec![];
    match dic_torrent.get(&ANNOUNCE_LIST.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list_of_lists)) => {
            for list in list_of_lists {
                match list {
                    ValuesBencoding::List(list_values) => {
                        let mut list_shuffle = list_values.clone();
                        list_shuffle.shuffle(&mut thread_rng());
                        for value in list_shuffle {
                            match value {
                                ValuesBencoding::String(url) => {
                                    list_tracker_url.push(vec_u8_to_string(&url))
                                }
                                _ => {
                                    return Err(TorrentFileDataError::Format(Section::TrackerList))
                                }
                            }
                        }
                    }
                    _ => return Err(TorrentFileDataError::Format(Section::TrackerList)),
                }
            }
            Ok(list_tracker_url)
        }
        _ => Ok(vec![]),
    }
}

fn init_info(dic_torrent: &DicValues) -> Result<DicValues, TorrentFileDataError> {
    match dic_torrent.get(&INFO.as_bytes().to_vec()) {
        Some(ValuesBencoding::Dic(dic_info)) => Ok(dic_info.clone()),
        Some(_) => Err(TorrentFileDataError::Format(Section::Info)),
        None => Err(TorrentFileDataError::NotFound(Section::Info)),
    }
}

fn init_info_hash(dic_info: &DicValues) -> Result<Vec<u8>, TorrentFileDataError> {
    //Paso info a bencoding
    let vec_info = bencoding::encoder::from_dic(dic_info.clone());
    //Le aplico SHA-1 a el info bencodeado
    let mut hasher = Sha1::new();
    hasher.update(vec_info);
    let result = hasher.finalize();
    let vec_sha1 = result.as_slice().to_vec();

    Ok(vec_sha1)
}

fn init_piece_length(dic_info: &DicValues) -> Result<u64, TorrentFileDataError> {
    match dic_info.get(&PIECE_LENGTH.as_bytes().to_vec()) {
        Some(ValuesBencoding::Integer(length)) => {
            Ok(u64::try_from(*length)
                .map_err(|_e| TorrentFileDataError::Creation(Section::Length))?)
        }
        Some(_) => Err(TorrentFileDataError::Format(Section::PieceLength)),
        None => Err(TorrentFileDataError::NotFound(Section::PieceLength)),
    }
}

fn init_size_files_single(dic_info: &DicValues) -> Result<i64, TorrentFileDataError> {
    //Si es single file solo tomo el valor de length
    if let Some(ValuesBencoding::Integer(length)) = dic_info.get(&LENGTH.as_bytes().to_vec()) {
        Ok(*length)
    } else {
        Err(TorrentFileDataError::NotFound(Section::Length))
    }
}

fn init_size_files_multiple(dic_info: &DicValues) -> Result<Vec<i64>, TorrentFileDataError> {
    //Si es multiple file recorro la lista de diccionarios de todos los archivos y tomo sus lengths
    let mut vec_sizes = vec![];
    match dic_info.get(&FILES.as_bytes().to_vec()) {
        Some(ValuesBencoding::List(list_files)) => {
            for file in list_files {
                if let ValuesBencoding::Dic(dic_file) = file {
                    match dic_file.get(&LENGTH.as_bytes().to_vec()) {
                        Some(ValuesBencoding::Integer(length)) => vec_sizes.push(*length),
                        Some(_) => return Err(TorrentFileDataError::Format(Section::Length)),
                        None => return Err(TorrentFileDataError::NotFound(Section::Length)),
                    }
                }
            }
            Ok(vec_sizes)
        }
        Some(_) => Err(TorrentFileDataError::Format(Section::Files)),
        None => Err(TorrentFileDataError::NotFound(Section::Files)),
    }
}

fn init_total_length(files_data: &TargetFilesData) -> u64 {
    match files_data {
        TargetFilesData::SingleFile {
            file_name: _,
            file_length,
        } => *file_length,
        TargetFilesData::MultipleFiles {
            dir_name: _,
            list_of_files_data,
        } => {
            let mut total_size = 0;
            list_of_files_data
                .iter()
                .for_each(|file_data| total_size += file_data.file_length);
            total_size
        }
    }
}

fn init_total_amount_pieces(total_size: u64, piece_lenght: u64) -> usize {
    let mut total_amount_pieces = (total_size / piece_lenght) as usize;
    if total_size % piece_lenght > 0 {
        total_amount_pieces += 1;
    }
    total_amount_pieces
}

fn init_pieces(dic_info: &DicValues) -> Result<Vec<u8>, TorrentFileDataError> {
    match dic_info.get(&PIECES.as_bytes().to_vec()) {
        Some(ValuesBencoding::String(pieces)) => Ok(pieces.clone()),
        Some(_) => Err(TorrentFileDataError::Format(Section::Pieces)),
        None => Err(TorrentFileDataError::NotFound(Section::Pieces)),
    }
}

fn string_of_path(path: Vec<ValuesBencoding>) -> Result<String, TorrentFileDataError> {
    let mut return_str = String::new();
    for value in path {
        return_str.push('/');
        match value {
            ValuesBencoding::String(dir) => return_str.push_str(&vec_u8_to_string(&dir)),
            _ => return Err(TorrentFileDataError::Format(Section::Path)),
        }
    }
    Ok(return_str)
}

fn init_path(dic_info: &DicValues) -> Result<Vec<String>, TorrentFileDataError> {
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
                        Some(_) => return Err(TorrentFileDataError::Format(Section::Length)),
                        None => return Err(TorrentFileDataError::NotFound(Section::Length)),
                    }
                }
            }
            Ok(return_vec)
        }
        Some(_) => Err(TorrentFileDataError::Format(Section::Files)),
        None => Ok(vec![]),
    }
}

fn init_list_files_data(dic_info: &DicValues) -> Result<Vec<FileData>, TorrentFileDataError> {
    let mut list_files_data = vec![];
    let paths = init_path(dic_info)?;
    let sizes = init_size_files_multiple(dic_info)?;

    if paths.len() != sizes.len() {
        return Err(TorrentFileDataError::Format(Section::FilesData));
    }

    //esto con un iterador quedaria mejor
    for i in 0..paths.len() {
        let file_data = FileData {
            path: paths[i].clone(),
            file_length: sizes[i] as u64,
        };
        list_files_data.push(file_data);
    }

    Ok(list_files_data)
}

fn init_target_files_data(
    dic_info: &DicValues,
    name: String,
) -> Result<TargetFilesData, TorrentFileDataError> {
    if is_single_file(dic_info) {
        let result = TargetFilesData::SingleFile {
            file_name: name,
            file_length: init_size_files_single(dic_info)? as u64,
        };
        Ok(result)
    } else {
        let list_of_files_data = init_list_files_data(dic_info)?;
        let result = TargetFilesData::MultipleFiles {
            dir_name: name,
            list_of_files_data,
        };
        Ok(result)
    }
}

impl TorrentFileData {
    ///Funcion para crear un TorrentFileData, necesita que se le pase un HashMap que tenga Vec<u8> como clave
    /// y ValuesBencoding como valores con los campos requeridos de un archivo .torrent, en caso de que no
    /// contenga alguno o haya formatos distintos a los deseados se devolvera el error correspondiente
    ///
    pub fn new(dic_torrent: DicValues) -> Result<Self, TorrentFileDataError> {
        let info = init_info(&dic_torrent)?;
        let name = init_name(&info)?;
        let piece_length = init_piece_length(&info)?;
        let target_files_data = init_target_files_data(&info, name)?;
        let total_length = init_total_length(&target_files_data);
        let total_amount_of_pieces = init_total_amount_pieces(total_length, piece_length);

        Ok(TorrentFileData {
            url_tracker_main: init_tracker_main(&dic_torrent)?,
            url_tracker_list: init_tracker_list(&dic_torrent)?,
            sha1_info_hash: init_info_hash(&info)?,
            sha1_pieces: init_pieces(&info)?,
            piece_length,
            total_length,
            total_amount_of_pieces,
            target_files_data,
        })
    }

    ///Funcion que devuelve la url del tracker principal del Torrent
    ///
    pub fn get_tracker_main(&self) -> String {
        self.url_tracker_main.clone()
    }

    ///Funcion que va a devolver el info_hash, que es el campo info del .torrent bencodeado y encriptado mediante
    /// SHA-1
    ///
    pub fn get_info_hash(&self) -> Vec<u8> {
        self.sha1_info_hash.clone()
    }

    ///Funcion que devuelve el tamaño total de todos los archivos
    ///
    pub fn get_total_length(&self) -> u64 {
        self.total_length
    }

    ///Funcion que devuelve el largo que van a tener las piezas
    ///
    pub fn get_piece_length(&self) -> u64 {
        self.piece_length
    }

    ///Funcion que devuelve la cantidad total de piezas que va a tener el archivo
    ///
    pub fn get_total_amount_pieces(&self) -> usize {
        self.total_amount_of_pieces
    }

    ///Funcion que dado el numero de pieza devuelve su encriptacion en SHA-1
    ///
    pub fn get_piece_sha1(&self, piece_index: usize) -> Vec<u8> {
        let mut pieces_return = vec![];
        let mut long_sha1 = 20;
        let mut iterator = self.sha1_pieces.clone().into_iter();
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

    pub fn has_expected_info_hash(&self, info_hash: &[u8]) -> bool {
        info_hash == self.sha1_info_hash
    }

    pub fn get_torrent_representative_name(&self) -> String {
        match &self.target_files_data {
            TargetFilesData::SingleFile {
                file_name,
                file_length: _,
            } => {
                let tmp_name = Path::new(&file_name)
                    .file_stem()
                    .map_or(Some("unnamed"), OsStr::to_str)
                    .map_or("unnamed_torrent", |name| name);
                tmp_name.to_string()
            }
            TargetFilesData::MultipleFiles {
                dir_name,
                list_of_files_data: _,
            } => dir_name.to_string(),
        }
    }

    fn is_last_piece_index(&self, piece_index: usize) -> bool {
        self.total_amount_of_pieces - 1 == piece_index
    }

    pub fn calculate_piece_lenght(&self, piece_index: usize) -> Result<u64, TorrentFileDataError> {
        if self.is_last_piece_index(piece_index) {
            let std_piece_lenght = self.get_piece_length();
            let total_amount_pieces = u64::try_from(self.get_total_amount_pieces())
                .map_err(|_| TorrentFileDataError::Calculation(Section::Length))?;
            let total_length = self.get_total_length();

            Ok(total_length - (std_piece_lenght * (total_amount_pieces - 1)))
        } else {
            Ok(self.piece_length)
        }
    }

    fn is_any_spare_bit_set(&self, bitfield: &[PieceStatus]) -> bool {
        return bitfield
            .iter()
            .skip(self.get_total_amount_pieces())
            .any(|piece_status| *piece_status == PieceStatus::ValidAndAvailablePiece);
    }

    pub fn check_bitfield(&self, bitfield: &[PieceStatus]) -> Result<(), TorrentFileDataError> {
        if bitfield.len() < self.get_total_amount_pieces() {
            return Err(TorrentFileDataError::CheckingBitfield(
                "[TorrentFileDataError] The bitfield length is incorrect.".to_string(),
            ));
        }

        if self.is_any_spare_bit_set(bitfield) {
            return Err(TorrentFileDataError::CheckingBitfield(
                "[TorrentFileDataError] Some of the spare bits are set.".to_string(),
            ));
        }
        Ok(())
    }

    pub fn check_requested_block(
        &self,
        piece_index: usize,
        beginning_byte_index: u32,
        amount_of_bytes: u32,
    ) -> Result<(), TorrentFileDataError> {
        let piece_length = self.calculate_piece_lenght(piece_index)?;
        if u64::from(beginning_byte_index + amount_of_bytes) > piece_length {
            return Err(TorrentFileDataError::CheckingRequestBlock("[TorrentFileDataError] The requested amount of bytes does not match with piece lenght.".to_string()));
        }

        if amount_of_bytes > BLOCK_BYTES {
            return Err(TorrentFileDataError::CheckingRequestBlock(
                "[TorrentFileDataError] The requested amount of bytes is bigger than 2^14 bytes."
                    .to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::torrent::client::medatada_analyzer::read_torrent_file_to_dic;

    #[test]
    fn test_torrent_single_file_ok() -> Result<(), Box<dyn Error>> {
        //ubuntu-14.04.6-server-ppc64el.iso [un solo archivo y un solo tracker]
        let dir = "torrents_for_test/ubuntu-14.04.6-server-ppc64el.iso.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => return Err(Box::new(error)),
        };
        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => return Err(Box::new(error)),
        };
        let tracker_main = String::from("http://torrent.ubuntu.com:6969/announce");

        let pieces_length_expected = 524288;
        let total_size_expected = 600401920;
        let total_pieces_expected = 1146;

        assert_eq!(torrent.get_tracker_main(), tracker_main);
        assert_eq!(torrent.get_piece_length(), pieces_length_expected);
        assert_eq!(torrent.get_total_length(), total_size_expected);
        assert_eq!(torrent.get_total_amount_pieces(), total_pieces_expected);

        let first_id = 0;
        let last_id = torrent.get_total_amount_pieces() - 1;

        let first_piece = torrent.get_piece_sha1(first_id);
        let last_piece = torrent.get_piece_sha1(last_id);

        let long_sha1 = 20;
        let pos_last_piece = torrent.sha1_pieces.len() - long_sha1;

        assert_eq!(first_piece, torrent.sha1_pieces[..long_sha1]);
        assert_eq!(last_piece, torrent.sha1_pieces[pos_last_piece..]);
        Ok(())
    }

    #[test]
    fn test_torrent_multiple_file_ok() -> Result<(), Box<dyn Error>> {
        let dir = "torrents_for_test/big-buck-bunny.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => return Err(Box::new(error)),
        };

        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => return Err(Box::new(error)),
        };

        let tracker_main = String::from("udp://tracker.leechers-paradise.org:6969");
        let pieces_length_expected = 262144;
        let total_size_expected = 140 + 276134947 + 310380;
        let total_pieces_expected = 1055;

        assert_eq!(torrent.get_tracker_main(), tracker_main);
        assert_eq!(torrent.get_piece_length(), pieces_length_expected);
        assert_eq!(torrent.get_total_length(), total_size_expected);
        assert_eq!(torrent.get_total_amount_pieces(), total_pieces_expected);

        let first_id = 0;
        let last_id = torrent.get_total_amount_pieces() - 1;

        let first_piece = torrent.get_piece_sha1(first_id);
        let last_piece = torrent.get_piece_sha1(last_id);

        let long_sha1 = 20;
        let pos_last_piece = torrent.sha1_pieces.len() - long_sha1;

        assert_eq!(first_piece, torrent.sha1_pieces[..long_sha1]);
        assert_eq!(last_piece, torrent.sha1_pieces[pos_last_piece..]);
        Ok(())
    }
}
