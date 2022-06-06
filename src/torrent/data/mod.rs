//! # Modulo general de manejo de datos
//! Este modulo contiene todos los submodulos usados para funcionamiento e interpretacion general de archivos .torrent (metadata), estados de descarga y datos importantes durante comunicaciones (con trackers o peers).
//!

pub mod data_of_download;
pub mod medatada_analyzer;
pub mod peer_data_for_communication;
pub mod torrent_file_data;
pub mod tracker_response_data;
