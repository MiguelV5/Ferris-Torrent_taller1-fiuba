//! # Modulo general de manejo de torrents asumiendo rol de CLIENTE (LEECHER)
//! Este modulo contiene todos los submodulos usados para funcionamiento general de comunicación y guardado de bloques de un torrent.
//!

pub mod block_handler;
pub mod entry_files_management;
pub mod medatada_analyzer;
pub mod peers_communication;
pub mod pieces_assembling_handler;
pub mod tracker_communication;
