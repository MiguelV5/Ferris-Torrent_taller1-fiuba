//! # Modulo de data actual de descarga
//! Este modulo contiene las funciones encargadas de almacenar y verificar
//! el estado actual de una descarga de un torrent
//!

use crate::torrent::parsers::p2p::message::PieceStatus;

#[derive(PartialEq, Debug, Clone)]
/// Representa el estado de la descarga COMPLETA del torrent
pub enum StateOfDownload {
    Started,
    Completed,
    Stopped,
}

#[derive(PartialEq, Debug, Clone)]
/// Representa la data actual de como va la descarga de un torrent
pub struct DataOfDownload {
    //pub address: SocketAddr,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: StateOfDownload,
    pub pieces_availability: Vec<PieceStatus>,
}

impl DataOfDownload {
    /// Funcion que crea informacion inicial del estado de descarga de un
    /// torrent.
    pub fn new(size_torrent: u64, total_amount_pieces: usize) -> Self {
        let mut pieces_availability = Vec::with_capacity(total_amount_pieces);
        pieces_availability.resize(total_amount_pieces, PieceStatus::MissingPiece);

        DataOfDownload {
            uploaded: 0,
            downloaded: 0,
            left: size_torrent,
            event: StateOfDownload::Started,
            pieces_availability,
        }
    }

    /// Funcion que indica si una pieza de tal indice está faltante por descargar
    pub fn is_a_missing_piece(&self, piece_index: usize) -> bool {
        if let Some(piece_status) = self.pieces_availability.get(piece_index) {
            *piece_status == PieceStatus::MissingPiece
        } else {
            false
        }
    }

    /// Funcion que indica si una pieza de tal indice ya fue descargada correctamente
    pub fn is_a_valid_and_available_piece(&self, piece_index: usize) -> bool {
        if let Some(piece_status) = self.pieces_availability.get(piece_index) {
            *piece_status == PieceStatus::ValidAndAvailablePiece
        } else {
            false
        }
    }

    /// Funcion que reinicia la data de la descarga actual desde cero
    pub fn flush_data(&mut self, size_torrent: u64) {
        self.uploaded = 0;
        self.downloaded = 0;
        self.left = size_torrent;
        self.event = StateOfDownload::Started;

        for piece_status in &mut self.pieces_availability {
            if let PieceStatus::PartiallyDownloaded {
                downloaded_bytes: _,
            } = *piece_status
            {
                *piece_status = PieceStatus::MissingPiece;
            }
        }
    }
}
