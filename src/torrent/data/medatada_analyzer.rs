#![allow(dead_code)]
use crate::torrent::parsers::bencoding;
use crate::torrent::parsers::bencoding::values::{ErrorBencoding, ValuesBencoding};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Read;
use std::{fs::File, path::Path};

type ResultMetadata<T> = Result<T, MetadataError>;
type DicValues = HashMap<Vec<u8>, ValuesBencoding>;

const TORRENT: &str = "torrent";

#[derive(Debug, PartialEq)]
pub enum MetadataError {
    FileNotFound,
    IsNotTorrent,
    Reading,
    TransferToDic(ErrorBencoding),
}

/// Se encarga de leer la información del .torrent
/// Devuelve un String con la información del archivo leído, y se encuentra en formato Bencoding
///
pub fn read_torrent_file(filename: &str) -> ResultMetadata<Vec<u8>> {
    if !check_filename_extension_is_torrent(filename) {
        return Err(MetadataError::IsNotTorrent);
    }

    let mut file = match File::open(filename) {
        Ok(file_open) => file_open,
        Err(_) => return Err(MetadataError::FileNotFound),
    };

    let mut bytes_vec: Vec<u8> = Vec::new();

    if file.read_to_end(&mut bytes_vec).is_err() {
        return Err(MetadataError::Reading);
    }

    Ok(bytes_vec)
}

pub fn read_torrent_file_to_dic(filename: &str) -> ResultMetadata<DicValues> {
    let metadata = read_torrent_file(filename)?;
    match bencoding::decoder::from_torrent_to_dic(metadata) {
        Ok(dic) => Ok(dic),
        Err(error) => Err(MetadataError::TransferToDic(error)),
    }
}

fn check_filename_extension_is_torrent(filename: &str) -> bool {
    let extension = Path::new(filename).extension().and_then(OsStr::to_str);
    Some(TORRENT) == extension
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::torrent::parsers::bencoding;
    #[test]
    fn read_torrent1_ok() {
        let file_dir = "torrents_for_test/ubuntu-22.04-desktop-amd64.iso.torrent";
        match read_torrent_file(file_dir) {
            Ok(torrent_metadata) => {
                match bencoding::decoder::from_torrent_to_dic(torrent_metadata.clone()) {
                    Ok(dic_torrent) => {
                        let to_bencoding = bencoding::encoder::from_dic(dic_torrent);
                        assert_eq!(torrent_metadata, to_bencoding);
                    }
                    Err(error) => panic!("ErrorBencoding: {:?}", error),
                }
            }
            Err(error) => panic!("MetadataError: {:?}", error),
        }
    }
    #[test]
    fn read_torrent2_ok() {
        let file_dir = "torrents_for_test/big-buck-bunny.torrent";
        match read_torrent_file(file_dir) {
            Ok(torrent_metadata) => {
                match bencoding::decoder::from_torrent_to_dic(torrent_metadata.clone()) {
                    Ok(dic_torrent) => {
                        let to_bencoding = bencoding::encoder::from_dic(dic_torrent);
                        assert_eq!(torrent_metadata, to_bencoding);
                    }
                    Err(error) => panic!("ErrorBencoding: {:?}", error),
                }
            }
            Err(error) => panic!("MetadataError: {:?}", error),
        }
    }
    #[test]
    fn read_torrent3_ok() {
        let file_dir = "torrents_for_test/ubuntu-14.04.6-server-ppc64el.iso.torrent";
        match read_torrent_file(file_dir) {
            Ok(torrent_metadata) => {
                match bencoding::decoder::from_torrent_to_dic(torrent_metadata.clone()) {
                    Ok(dic_torrent) => {
                        let to_bencoding = bencoding::encoder::from_dic(dic_torrent);
                        assert_eq!(torrent_metadata, to_bencoding);
                    }
                    Err(error) => panic!("ErrorBencoding: {:?}", error),
                }
            }
            Err(error) => panic!("MetadataError: {:?}", error),
        }
    }
    #[test]
    fn read_no_file() {
        let file_dir = "torrents_for_test/torrent_no_existente.torrent";
        let metadata = read_torrent_file(file_dir);
        assert_eq!(metadata, Err(MetadataError::FileNotFound))
    }
    #[test]
    fn read_file_other_format() {
        let file_dir = "torrents_for_test/torrent_no_existente.iso";
        let metadata = read_torrent_file(file_dir);
        assert_eq!(metadata, Err(MetadataError::IsNotTorrent))
    }

    // Creo que a partir de aca podriamos hacer tests de integracion en un directorio tests en la raiz del proyecto;
    // más que nada porque podriamos testear la lectura del .torrent + su decodificacion a Diccionario.
    // En ese sentido creo que un mejor nombre para este archivo si seria algo de tipo metadata_analyzer como habia propuesto Erick antes,
    // pues aca falta toda esa logica de decodificacion y etc. (Ahí se lo cambié:  metadata_reader --> metadata_analyzer)
}
