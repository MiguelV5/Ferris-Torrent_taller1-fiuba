//! # Modulo de manejo general de almacenamiento de BLOQUES
//! Este modulo contiene las funciones encargadas de almacenar bloques
//! recibidos de mensajes de tipo "Piece" en medio de interacciones individuales
//! con peers.

use crate::torrent::data::torrent_file_data::TorrentFileData;

use core::fmt;
use log::info;
use sha1::{Digest, Sha1};
use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::prelude::FileExt,
};

/// Representa un error de manejo de almacenamiento de bloque.
#[derive(PartialEq, Debug, Clone)]
pub enum BlockHandlerError {
    StoringBlock(String),
    CheckingSha1Piece(String),
    IncorrectSha1Piece(String),
    GettingBlock(String),
}

impl fmt::Display for BlockHandlerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for BlockHandlerError {}

///
/// Funcion que, dado un bloque descargado de una comunicacion individual con
/// un peer, escribe en disco (en un path correspondiente a su PIEZA respectiva)
/// dicho bloque
///
pub fn store_block(block: &[u8], piece_index: usize, path: &str) -> Result<(), BlockHandlerError> {
    let file_name = format!("temp/{}/piece_{}", path, piece_index);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_name)
        .map_err(|err| BlockHandlerError::StoringBlock(format!("{}", err)))?;
    file.write_all(block)
        .map_err(|err| BlockHandlerError::StoringBlock(format!("{}", err)))?;
    Ok(())
}

fn read_a_piece(piece_index: usize, path: &str) -> Result<Vec<u8>, BlockHandlerError> {
    let file_name = format!("temp/{}/piece_{}", path, piece_index);
    let mut file = OpenOptions::new()
        .create(false)
        .read(true)
        .open(file_name)
        .map_err(|err| BlockHandlerError::CheckingSha1Piece(format!("{}", err)))?;

    let mut piece = Vec::new();
    file.read_to_end(&mut piece)
        .map_err(|err| BlockHandlerError::CheckingSha1Piece(format!("{}", err)))?;

    Ok(piece)
}

fn get_sha1(buffer: &[u8]) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(&buffer);
    hasher.finalize().as_slice().to_vec()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|a| format!("{:02x}", a)).collect()
}

///
/// Funcion que busca en el archivo de piezas a la pieza correspondiente
/// segun el indice dado, le calcula sha1 y verifica que sea el mismo que estaba
/// contenido en el archivo .torrent dado.
///
pub fn check_sha1_piece(
    torrent_file_data: &TorrentFileData,
    piece_index: usize,
    path: &str,
) -> Result<(), BlockHandlerError> {
    let piece = read_a_piece(piece_index, path)?;
    let piece_sha1 = get_sha1(&piece);

    let expected_piece_sha1 = torrent_file_data.get_piece_sha1(piece_index);

    info!(
        "\n    Hash SHA1 esperado: {:?}",
        to_hex(&expected_piece_sha1)
    );
    info!(
        "\n    Hash SHA1 obtenido por pieza descargada: {:?}",
        to_hex(&piece_sha1)
    );

    if piece_sha1 != expected_piece_sha1 {
        fs::remove_file(format!("temp/{}/piece_{}", path, piece_index))
            .map_err(|err| BlockHandlerError::CheckingSha1Piece(format!("{}", err)))?;
        Err(BlockHandlerError::CheckingSha1Piece(
            "The downloaded piece does not pass the sha1 verification.".to_string(),
        ))
    } else {
        Ok(())
    }
}

///
/// Funcion encargada de obtener una bloque previamente descargador dado su numero de pieza, el byte de comienzo
/// y la cantidad de bytes requeridos. Ademas de estos tres datos mencionados anteriormente, se debe suministrar la ruta en donde se
/// encuentran guardadas las piezas temporalmente.
///
pub fn get_block(
    piece_index: u32,
    beginning_byte_index: u32,
    amount_of_bytes: u32,
    path: &str,
) -> Result<Vec<u8>, BlockHandlerError> {
    let amount_of_bytes = amount_of_bytes
        .try_into()
        .map_err(|err| BlockHandlerError::GettingBlock(format!("{}", err)))?;
    let file_name = format!("temp/{}/piece_{}", path, piece_index);

    let mut block = vec![0; amount_of_bytes];

    let file = OpenOptions::new()
        .read(true)
        .open(file_name)
        .map_err(|err| BlockHandlerError::GettingBlock(format!("{}", err)))?;
    file.read_exact_at(&mut block, beginning_byte_index.into())
        .map_err(|err| BlockHandlerError::GettingBlock(format!("{}", err)))?;
    Ok(block)
}

#[cfg(test)]
mod test_block_handler {
    use super::*;
    use std::{
        error::Error,
        fs::{self, File},
        io::Read,
    };

    mod test_get_block {
        use super::*;

        #[test]
        fn first_block_can_be_gotten_ok() -> Result<(), Box<dyn Error>> {
            let expected_block = [1; 16].to_vec();
            let piece_index = 0;
            let beginning_byte_index = 0;
            let amount_of_bytes = 16;

            let path = "test_block_handler/get_block_1".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            store_block(&expected_block, piece_index, &path)?;
            let received_block = get_block(
                piece_index.try_into()?,
                beginning_byte_index,
                amount_of_bytes,
                &path,
            )?;

            assert_eq!(expected_block, received_block);

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn last_block_can_be_gotten_ok() -> Result<(), Box<dyn Error>> {
            let block_0 = [0; 16].to_vec();
            let block_1 = [1; 16].to_vec();
            let block_2 = [2; 16].to_vec();
            let piece_index = 0;
            let beginning_byte_index = 32;
            let amount_of_bytes = 16;

            let path = "test_block_handler/get_block_2".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            store_block(&block_0, piece_index, &path)?;
            store_block(&block_1, piece_index, &path)?;
            store_block(&block_2, piece_index, &path)?;

            let received_block = get_block(
                piece_index.try_into()?,
                beginning_byte_index,
                amount_of_bytes,
                &path,
            )?;

            assert_eq!(block_2, received_block);

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn all_blocks_can_be_gotten_ok() -> Result<(), Box<dyn Error>> {
            let block_0 = [0; 16].to_vec();
            let block_1 = [1; 16].to_vec();
            let block_2 = [2; 16].to_vec();
            let piece_index = 0;
            let beginning_byte_index = 0;
            let amount_of_bytes = 16;

            let path = "test_block_handler/get_block_3".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            store_block(&block_0, piece_index, &path)?;
            store_block(&block_1, piece_index, &path)?;
            store_block(&block_2, piece_index, &path)?;

            let received_block_0 = get_block(
                piece_index.try_into()?,
                beginning_byte_index,
                amount_of_bytes,
                &path,
            )?;

            let beginning_byte_index = beginning_byte_index + 16;

            let received_block_1 = get_block(
                piece_index.try_into()?,
                beginning_byte_index,
                amount_of_bytes,
                &path,
            )?;

            let beginning_byte_index = beginning_byte_index + 16;

            let received_block_2 = get_block(
                piece_index.try_into()?,
                beginning_byte_index,
                amount_of_bytes,
                &path,
            )?;

            assert_eq!(block_0, received_block_0);
            assert_eq!(block_1, received_block_1);
            assert_eq!(block_2, received_block_2);

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }
    }

    mod test_store_block {
        use super::*;

        #[test]
        fn one_block_can_be_stored() -> Result<(), Box<dyn Error>> {
            let block = [1; 16].to_vec();
            let piece_index = 0;

            let path = "test_block_handler/store_block_1".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            store_block(&block, piece_index, &path)?;

            let mut file = File::open(format!("temp/{}/piece_{}", path, piece_index))?;
            let mut file_block: Vec<u8> = Vec::new();

            file.read_to_end(&mut file_block)?;

            assert_eq!(block, file_block);

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn multiple_blocks_can_be_stored() -> Result<(), Box<dyn Error>> {
            let block_0 = [0; 16].to_vec();
            let mut block_1 = [1; 16].to_vec();
            let mut block_2 = [2; 16].to_vec();
            let piece_index = 0;

            let path = "test_block_handler/store_block_2".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            store_block(&block_0, piece_index, &path)?;
            store_block(&block_1, piece_index, &path)?;
            store_block(&block_2, piece_index, &path)?;

            let mut expected_block = block_0;
            expected_block.append(&mut block_1);
            expected_block.append(&mut block_2);

            let mut file = File::open(format!("temp/{}/piece_{}", path, piece_index))?;
            let mut file_block: Vec<u8> = Vec::new();

            file.read_to_end(&mut file_block)?;

            assert_eq!(expected_block, file_block);

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }
    }
}
