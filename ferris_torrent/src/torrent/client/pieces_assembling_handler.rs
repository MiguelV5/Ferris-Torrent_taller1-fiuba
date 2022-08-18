//! # Modulo de ensamblado de piezas.
//! Este modulo contiene las funciones encargadas de controlar todo el ensamblado de piezas a un único archivo.
//!

use crate::torrent::data::torrent_file_data::{TargetFilesData, TorrentFileData};
use core::fmt;
use log::info;
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
};

#[derive(Debug, PartialEq, Eq)]
pub enum PiecesAssemblerError {
    SetUpDownloadDirectory(String),
    ReadingAPieceFile(String),
    WritingTargetFile(String),
}

impl fmt::Display for PiecesAssemblerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for PiecesAssemblerError {}

fn set_up_download_dir(
    download_dir_path: &str,
    torrent_file_data: &TorrentFileData,
) -> Result<File, PiecesAssemblerError> {
    let single_file_name = match &torrent_file_data.target_files_data {
        TargetFilesData::MultipleFiles {
            dir_name: _,
            list_of_files_data: _,
        } => {
            // ACCION TEMPORAL:
            return Err(PiecesAssemblerError::SetUpDownloadDirectory(
                "Piece Assembling for Multiple file torrents not supported yet".to_string(),
            ));
        }
        TargetFilesData::SingleFile {
            file_name,
            file_length: _,
        } => file_name.clone(),
    };

    info!("Creando directorio para guardar ensamblar la descarga a partir de las piezas.");
    let _result = fs::remove_dir_all(download_dir_path);
    fs::create_dir(download_dir_path)
        .map_err(|err| PiecesAssemblerError::SetUpDownloadDirectory(format!("{}", err)))?;

    let target_file_path = format!("{}/{}", download_dir_path, single_file_name);
    let target_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(target_file_path)
        .map_err(|err| PiecesAssemblerError::SetUpDownloadDirectory(format!("{}", err)))?;
    info!("Creacion de directorio exitosa. Preparando para ensamblar descarga.");
    Ok(target_file)
}

fn open_piece_file_at(
    piece_index: usize,
    torrent_file_data: &TorrentFileData,
) -> Result<(String, File), PiecesAssemblerError> {
    let torrent_representative_name = torrent_file_data.get_torrent_representative_name();
    let corresponding_piece_path =
        format!("temp/{}/piece_{}", torrent_representative_name, piece_index);

    let piece_file = fs::File::open(&corresponding_piece_path).map_err(|err| {
        PiecesAssemblerError::ReadingAPieceFile(format!(
            "File that failed:  {}\n    {}",
            corresponding_piece_path, err
        ))
    })?;

    Ok((corresponding_piece_path, piece_file))
}

fn remove_all_assembled_data(piece_file_path: &str) {
    let _rm_result = fs::remove_dir_all(piece_file_path);
}

///
/// FUNCION PRINCIPAL
/// Funcion encargada de el ensamblado de todas las piezas en un único archivo, siendo que actualmente se soporta
/// la descarga de archivos del tipo Single File.
///
pub fn assemble_all_completed_pieces(
    desired_path_for_target: String,
    torrent_file_data: &TorrentFileData,
) -> Result<(), PiecesAssemblerError> {
    let torrent_representative_name = torrent_file_data.get_torrent_representative_name();

    let mut target_single_file = set_up_download_dir(&desired_path_for_target, torrent_file_data)?;

    let mut current_piece_index = 0;
    let mut current_piece_to_transfer: Vec<u8> =
        Vec::with_capacity(torrent_file_data.piece_length as usize);

    while current_piece_index < torrent_file_data.get_total_amount_pieces() {
        let (piece_file_path, mut piece_file) =
            open_piece_file_at(current_piece_index, torrent_file_data)?;

        current_piece_to_transfer.clear();

        let amount_of_bytes_read = piece_file
            .read_to_end(&mut current_piece_to_transfer)
            .map_err(|err| PiecesAssemblerError::ReadingAPieceFile(err.to_string()))?;

        if amount_of_bytes_read == 0 {
            current_piece_index += 1;
            continue;
        }
        let writing_result = target_single_file
            .write_all(&current_piece_to_transfer)
            .map_err(|err| PiecesAssemblerError::WritingTargetFile(err.to_string()));

        if let Err(err) = writing_result {
            remove_all_assembled_data(&piece_file_path);
            return Err(err);
        }

        current_piece_index += 1;
    }

    info!(
        "Ensamblado de torrent exitoso: {}.",
        torrent_representative_name
    );
    Ok(())
}

#[cfg(test)]
mod tests_pieces_handler {
    use std::{error::Error, io};

    use super::*;

    //====================================================

    fn create_single_piece_file_for_test(
        torrent_file_data: &TorrentFileData,
    ) -> Result<(), io::Error> {
        let current_temp_path = "temp/test_assembling_from_one_piece";
        let _ = fs::remove_dir_all(current_temp_path);
        fs::create_dir(current_temp_path)?;

        let bytes = vec![0u8, 0, 0];

        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!(
                "temp/{}/piece_0",
                torrent_file_data.get_torrent_representative_name()
            ))?
            .write_all(&bytes)?;
        Ok(())
    }

    fn create_mock_torrent_file_with_one_already_downloaded_piece(
    ) -> Result<TorrentFileData, io::Error> {
        let torrent_file_data = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: "test_assembling_from_one_piece.txt".to_string(),
                file_length: 3,
            },
            sha1_pieces: vec![], // No necesario para el test
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            sha1_info_hash: vec![],
            piece_length: 3,
            total_amount_of_pieces: 1,
            total_length: 3,
        };

        create_single_piece_file_for_test(&torrent_file_data)?;

        Ok(torrent_file_data)
    }

    fn create_multiple_piece_files_for_test(
        torrent_file_data: &TorrentFileData,
    ) -> Result<(), io::Error> {
        let current_temp_path = "temp/test_assembling_from_multiple_pieces";
        let _ = fs::remove_dir_all(current_temp_path);
        fs::create_dir(current_temp_path)?;

        let bytes_piece_0 = vec![0u8, 0, 0];
        let bytes_piece_1 = vec![0, 0, 1];
        let bytes_piece_2 = vec![0, 0, 2];
        let bytes_piece_3 = vec![0, 0, 3];
        let bytes_piece_4 = vec![0, 4];

        let pieces_bytes = vec![
            bytes_piece_0,
            bytes_piece_1,
            bytes_piece_2,
            bytes_piece_3,
            bytes_piece_4,
        ];

        for i in 0..5 {
            fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(format!(
                    "temp/{}/piece_{}",
                    torrent_file_data.get_torrent_representative_name(),
                    i
                ))?
                .write_all(&pieces_bytes[i])?;
        }
        Ok(())
    }

    fn create_mock_torrent_file_with_multiple_already_downloaded_pieces(
    ) -> Result<TorrentFileData, io::Error> {
        let torrent_file_data = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: "test_assembling_from_multiple_pieces.txt".to_string(),
                file_length: 4 * (3) + 1 * (2),
            },
            sha1_pieces: vec![], // No necesario para el test
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            sha1_info_hash: vec![],
            piece_length: 3,
            total_amount_of_pieces: 5,
            total_length: 4 * (3) + 1 * (2), // 4 piezas de 3 bytes c/u  y ultima de 2 bytes.
        };

        create_multiple_piece_files_for_test(&torrent_file_data)?;

        Ok(torrent_file_data)
    }

    //====================================================

    #[test]
    fn torrent_pieces_are_successfully_assembled_from_a_unique_piece_ok(
    ) -> Result<(), Box<dyn Error>> {
        let torrent_file_data = create_mock_torrent_file_with_one_already_downloaded_piece()?;
        let desired_path_for_target = String::from("temp/target_mock_from_one_piece");
        let separated_pieces_path = String::from("temp/test_assembling_from_one_piece");
        assert_eq!(
            Ok(()),
            assemble_all_completed_pieces(desired_path_for_target.clone(), &torrent_file_data)
        );

        let expected_reading_from_assembled_file = vec![0u8, 0, 0];
        let mut read_from_target = vec![];

        fs::File::open(format!(
            "{}/{}.txt",
            &desired_path_for_target,
            torrent_file_data.get_torrent_representative_name()
        ))?
        .read_to_end(&mut read_from_target)?;

        assert_eq!(expected_reading_from_assembled_file, read_from_target);
        assert_eq!(
            torrent_file_data.get_total_length() as usize,
            read_from_target.len()
        );

        let _ = fs::remove_dir_all(desired_path_for_target);
        let _ = fs::remove_dir_all(separated_pieces_path);

        Ok(())
    }

    #[test]
    fn torrent_pieces_are_successfully_assembled_from_multiple_pieces_ok(
    ) -> Result<(), Box<dyn Error>> {
        let torrent_file_data = create_mock_torrent_file_with_multiple_already_downloaded_pieces()?;
        let desired_path_for_target = String::from("temp/target_mock_from_multiple_pieces");
        let separated_pieces_path = String::from("temp/test_assembling_from_multiple_pieces");
        assert_eq!(
            Ok(()),
            assemble_all_completed_pieces(desired_path_for_target.clone(), &torrent_file_data)
        );

        let expected_reading_from_assembled_file = vec![0u8, 0, 0, 0, 0, 1, 0, 0, 2, 0, 0, 3, 0, 4];
        let mut read_from_target = vec![];

        fs::File::open(format!(
            "{}/{}.txt",
            &desired_path_for_target,
            torrent_file_data.get_torrent_representative_name()
        ))?
        .read_to_end(&mut read_from_target)?;

        assert_eq!(expected_reading_from_assembled_file, read_from_target);
        assert_eq!(
            torrent_file_data.get_total_length() as usize,
            read_from_target.len()
        );

        let _ = fs::remove_dir_all(desired_path_for_target);
        let _ = fs::remove_dir_all(separated_pieces_path);

        Ok(())
    }
}
