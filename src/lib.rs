#![allow(unused_variables)]
pub mod torrent;
use std::error::Error;
use torrent::data::{
    medatada_analyzer::read_torrent_file_to_dic, torrent_file_data::TorrentFileData,
};

use crate::torrent::client::tracker_comunication::connect::init_communication;

pub fn start_torrent_process(torrent_path: &str) -> Result<(), Box<dyn Error>> {
    let torrent_dic = read_torrent_file_to_dic(torrent_path)?;
    println!("Torrent leído ok...");
    let torrent_data = TorrentFileData::new(torrent_dic)?;
    println!("Torrent formato ok...");
    println!("Empezando comunicación con el tracker...");
    let response = init_communication(torrent_data)?;
    println!("Comunicación con el tracker ok...");
    println!("Inicio comunicación con los peers...");

    Ok(())
}
