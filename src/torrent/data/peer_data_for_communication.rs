//! # Modulo de data de un peer para comunicacion P2P
//! Este modulo contiene las estructuras encargadas de almacenar la
//! información de un peer obtenida durante comunicación P2P
//!

use crate::torrent::parsers::p2p::message::PieceStatus;

#[derive(PartialEq, Debug, Clone)]
/// Representa la info importante de un peer al comunicarse con él de forma
/// directa por sockets
pub struct PeerDataForP2PCommunication {
    pub peer_id: Vec<u8>,
    pub pieces_availability: Option<Vec<PieceStatus>>,
    pub am_choking: bool,
    pub am_interested: bool,
    pub peer_choking: bool,
    //pub peer_interested: bool,
    //...
}

impl PeerDataForP2PCommunication {
    pub fn new(peer_id: Vec<u8>) -> Self {
        PeerDataForP2PCommunication {
            pieces_availability: None,
            peer_id,
            am_interested: false,
            am_choking: true,
            peer_choking: true,
        }
    }
}
