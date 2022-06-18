//!
//!
//!

// use super::{
//     client::peers_comunication::handler::BLOCK_BYTES, data::torrent_file_data::TorrentFileData,
// };
// use std::{
//     fs::{self, DirEntry},
//     io::{self, Read},
// };

// pub fn assemble_all_completed_pieces(
//     desired_path_for_target: &str,
//     torrent_file_data: &TorrentFileData,
// ) -> io::Result<()> {
//     let torrent_representative_name = torrent_file_data.get_torrent_representative_name();
//     let previous_temp_dir = format!("temp/{}", &torrent_representative_name);
//     let download_path = format!(
//         "downloads/{}/{}",
//         &torrent_representative_name, desired_path_for_target
//     );

//     let mut current_block: Vec<u8> = Vec::with_capacity(BLOCK_BYTES as usize);
//     let entries_iter = fs::read_dir(&previous_temp_dir)?;
//     for entry in entries_iter {
//         let piece_file_as_entry = entry?;

//         let mut piece_file = fs::File::open(piece_file_as_entry.path())?;
//         for result_of_amount_of_bytes_read in piece_file.read(&mut current_block) {
//             match result_of_amount_of_bytes_read {
//                 Ok(0) => break,
//                 Ok(amount_read) =>
//             }

//         }
//     }

//     Ok(())
// }

// WIP
