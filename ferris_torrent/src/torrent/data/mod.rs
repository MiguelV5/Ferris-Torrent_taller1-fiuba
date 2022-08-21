//! # Modulo general de manejo de datos
//! Este modulo contiene todos los submodulos usados para funcionamiento e interpretacion general de archivos .torrent (metadata), estados de descarga y datos importantes durante comunicaciones (con trackers o peers).
//!

pub mod config_file_data;
pub mod peer_data_for_communication;
pub mod torrent_status;
pub mod tracker_response_data;
