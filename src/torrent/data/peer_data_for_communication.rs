//! # Modulo de data de un peer para comunicacion P2P
//! Este modulo contiene las estructuras encargadas de almacenar la
//! información de un peer obtenida durante comunicación P2P
//!

use crate::torrent::parsers::p2p::message::PieceStatus;

use super::torrent_file_data::TorrentFileData;

#[derive(PartialEq, Debug, Clone)]
pub enum PeerDataForP2PCommunicationError {
    InvalidPieceIndexAtBitfield(String),
}

#[derive(PartialEq, Debug, Clone)]
/// Representa la info importante de un peer al comunicarse con él de forma
/// directa por sockets
pub struct PeerDataForP2PCommunication {
    pub peer_id: Vec<u8>,
    pub pieces_availability: Vec<PieceStatus>,
    pub am_choking: bool,
    pub am_interested: bool,
    pub peer_choking: bool,
    pub peer_interested: bool,
}

fn generate_empty_bitfield(total_amount_pieces: usize) -> Vec<PieceStatus> {
    let mut pieces_availability = Vec::with_capacity(total_amount_pieces);
    pieces_availability.resize(total_amount_pieces, PieceStatus::MissingPiece);
    pieces_availability
}

impl PeerDataForP2PCommunication {
    pub fn new(torrent_file_data: &TorrentFileData, peer_id: Vec<u8>) -> Self {
        let total_amount_pieces = torrent_file_data.get_total_amount_pieces();
        PeerDataForP2PCommunication {
            peer_id,
            pieces_availability: generate_empty_bitfield(total_amount_pieces),
            am_choking: true,
            am_interested: false,
            peer_choking: true,
            peer_interested: false,
        }
    }

    pub fn update_piece_status(
        &mut self,
        piece_index: usize,
        new_status: PieceStatus,
    ) -> Result<(), PeerDataForP2PCommunicationError> {
        if let Some(piece_status) = self.pieces_availability.get_mut(piece_index) {
            *piece_status = new_status;
            Ok(())
        } else {
            Err(PeerDataForP2PCommunicationError::InvalidPieceIndexAtBitfield(
                "[PeerDataForP2PCommunicationError] Invalid indexation of pieces availability (bitfield).".to_string(),
            ))
        }
    }

    pub fn update_pieces_availability(&mut self, new_pieces_availability: Vec<PieceStatus>) {
        self.pieces_availability = new_pieces_availability;
    }
}
