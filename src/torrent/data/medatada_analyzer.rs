#![allow(dead_code)]
use std::error::Error;
use std::ffi::OsStr;
use std::io::Read;
use std::{fs::File, path::Path};

/// Se encarga de leer la información del .torrent
/// Devuelve un String con la información del archivo leído, y se encuentra en formato Bencoding
///
pub fn read_torrent_file(filename: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut file = File::open(filename)?;
    if !check_filename_extension_is_torrent(filename) {
        return Err("Not a torrent file".into());
    }
    let mut bytes_vec: Vec<u8> = Vec::new();
    file.read_to_end(&mut bytes_vec)?;
    Ok(bytes_vec)
    //let bytes_slice: &[u8] = &bytes_vec;
    //let raw_metadata_bytes = String::from_utf8_lossy(bytes_slice).to_string();
    //Ok(raw_metadata_bytes)
}

fn check_filename_extension_is_torrent(filename: &str) -> bool {
    let extension = Path::new(filename).extension().and_then(OsStr::to_str);
    Some("torrent") == extension
}

#[cfg(test)]
mod tests {

    use crate::torrent::parsers::bencoding::decoder::to_dic;

    use super::*;
    #[test]
    fn read_some_torrent_ok() {
        let mut torrent_dir = std::env::current_dir().unwrap().to_owned();
        torrent_dir.push("torrents_for_test/big-buck-bunny.torrent");

        let metadata = read_torrent_file(torrent_dir.to_str().unwrap()).unwrap();
        assert!(metadata.len() > 0); //Para cambiar luego
        let dic_torrent = to_dic(metadata);
        assert!(dic_torrent.is_ok());
    }
    // Creo que a partir de aca podriamos hacer tests de integracion en un directorio tests en la raiz del proyecto;
    // más que nada porque podriamos testear la lectura del .torrent + su decodificacion a Diccionario.
    // En ese sentido creo que un mejor nombre para este archivo si seria algo de tipo metadata_analyzer como habia propuesto Erick antes,
    // pues aca falta toda esa logica de decodificacion y etc. (Ahí se lo cambié:  metadata_reader --> metadata_analyzer)
}
