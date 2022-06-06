#![allow(dead_code)]
use crate::torrent::parsers::p2p::message::PieceStatus;

#[derive(PartialEq, Debug, Clone)]
pub enum StateOfDownload {
    Started,
    Completed,
    Stopped,
}

#[derive(PartialEq, Debug, Clone)]
pub struct DataOfDownload {
    //pub address: SocketAddr,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: StateOfDownload,
    pub pieces_availability: Vec<PieceStatus>,
    //..
}

impl DataOfDownload {
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

    pub fn is_a_missing_piece(&self, piece_index: usize) -> bool {
        if let Some(piece_status) = self.pieces_availability.get(piece_index) {
            *piece_status == PieceStatus::MissingPiece
        } else {
            false
        }
    }

    pub fn is_a_valid_and_available_piece(&self, piece_index: usize) -> bool {
        if let Some(piece_status) = self.pieces_availability.get(piece_index) {
            *piece_status == PieceStatus::ValidAndAvailablePiece
        } else {
            false
        }
    }

    pub fn flush_data(&mut self, size_torrent: u64) {
        self.uploaded = 0;
        self.downloaded = 0;
        self.left = size_torrent;
        self.event = StateOfDownload::Started;

        //esto se puede hacer con un iterador
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
