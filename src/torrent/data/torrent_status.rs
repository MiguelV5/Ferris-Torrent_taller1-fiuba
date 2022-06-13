//! # Modulo de data actual de descarga
//! Este modulo contiene l&as funciones encargadas de almacenar y verificar
//! el estado actual de una descarga de un torrent
//!

use crate::torrent::{
    client::peers_comunication::msg_logic_control::BLOCK_BYTES, local_peer::LocalPeer,
    parsers::p2p::message::PieceStatus,
};

use super::torrent_file_data::TorrentFileData;

#[derive(PartialEq, Debug, Clone)]
/// Representa el estado de la descarga COMPLETA del torrent
pub enum TorrentStatusError {
    UpdatingPieceStatus(String),
    CalculatingBeginningByteIndex(String),
    CalculatingAmountOfBytes(String),
}

#[derive(PartialEq, Debug, Clone)]
/// Representa el estado de la descarga COMPLETA del torrent
pub enum StateOfDownload {
    Started,
    Completed,
    Stopped,
}

#[derive(PartialEq, Debug, Clone)]
/// Representa la data actual de como va la descarga de un torrent
pub struct TorrentStatus {
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: StateOfDownload,
    pub pieces_availability: Vec<PieceStatus>,
}

impl TorrentStatus {
    /// Funcion que crea informacion inicial del estado de descarga de un
    /// torrent.
    ///
    pub fn new(size_torrent: u64, total_amount_pieces: usize) -> Self {
        let mut pieces_availability = Vec::with_capacity(total_amount_pieces);
        pieces_availability.resize(total_amount_pieces, PieceStatus::MissingPiece);

        TorrentStatus {
            uploaded: 0,
            downloaded: 0,
            left: size_torrent,
            event: StateOfDownload::Started,
            pieces_availability,
        }
    }

    /// Funcion que indica si una pieza de tal indice está faltante por descargar
    ///
    pub fn is_a_missing_piece(&self, piece_index: usize) -> bool {
        if let Some(piece_status) = self.pieces_availability.get(piece_index) {
            *piece_status == PieceStatus::MissingPiece
        } else {
            false
        }
    }

    /// Funcion que indica si una pieza de tal indice ya fue descargada correctamente
    ///
    pub fn is_a_valid_and_available_piece(&self, piece_index: usize) -> bool {
        if let Some(piece_status) = self.pieces_availability.get(piece_index) {
            *piece_status == PieceStatus::ValidAndAvailablePiece
        } else {
            false
        }
    }

    /// Funcion que reinicia la data de la descarga actual desde cero
    ///
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

    pub fn get_piece_status(&self, piece_index: usize) -> Option<&PieceStatus> {
        self.pieces_availability.get(piece_index)
    }

    pub fn get_pieces_availability(&self) -> Vec<PieceStatus> {
        self.pieces_availability.clone()
    }

    fn increment_downloaded_counter(&mut self, amount_of_bytes: u64) {
        self.downloaded += amount_of_bytes;
        self.left -= amount_of_bytes;
    }

    pub fn update_piece_status(
        &mut self,
        torrent_file_data: &TorrentFileData,
        piece_index: usize,
        beginning_byte_index: u32,
        amount_of_bytes: u32,
    ) -> Result<(), TorrentStatusError> {
        //Se la podria modularizar
        let piece_lenght = torrent_file_data
            .calculate_piece_lenght(piece_index)
            .map_err(|err| TorrentStatusError::UpdatingPieceStatus(format!("{:?}", err)))?;

        if let Some(piece_status) = self.pieces_availability.get_mut(piece_index as usize) {
            match piece_status {
                PieceStatus::MissingPiece => {
                    if piece_lenght == amount_of_bytes.into() {
                        *piece_status = PieceStatus::ValidAndAvailablePiece;
                    } else {
                        *piece_status = PieceStatus::PartiallyDownloaded {
                            downloaded_bytes: amount_of_bytes,
                        };
                    }
                    self.increment_downloaded_counter(amount_of_bytes.into())
                }
                PieceStatus::PartiallyDownloaded { downloaded_bytes } => {
                    let remaining_bytes =
                        piece_lenght - u64::from(*downloaded_bytes + amount_of_bytes);
                    if remaining_bytes == 0 {
                        *piece_status = PieceStatus::ValidAndAvailablePiece;
                    } else {
                        *piece_status = PieceStatus::PartiallyDownloaded {
                            downloaded_bytes: beginning_byte_index + amount_of_bytes,
                        };
                    }
                    self.increment_downloaded_counter(amount_of_bytes.into())
                }
                PieceStatus::ValidAndAvailablePiece => {
                    return Err(TorrentStatusError::UpdatingPieceStatus(
                        "[TorrentStatusError] The piece has already been completed.".to_string(),
                    ))
                }
            }
        };
        Ok(())
    }

    /// Funcion que busca una nueva pieza que quiera pedir posteriormente, y
    /// devuelve su indice
    ///
    pub fn look_for_a_missing_piece_index(&self, local_peer: &LocalPeer) -> Option<usize> {
        let (piece_index, _piece_status) =
            self.pieces_availability
                .iter()
                .enumerate()
                .find(|(piece_index, piece_status)| {
                    (**piece_status != PieceStatus::ValidAndAvailablePiece)
                        && local_peer
                            .external_peer_has_a_valid_and_available_piece_on_position(*piece_index)
                })?;
        Some(piece_index)
    }

    /// Funcion que calcula el byte inicial desde el cual
    /// se deberia pedir el siguiente bloque de una pieza
    ///
    pub fn calculate_beginning_byte_index(
        &self,
        piece_index: usize,
    ) -> Result<u32, TorrentStatusError> {
        match self.pieces_availability.get(piece_index)
        {
            Some(PieceStatus::PartiallyDownloaded { downloaded_bytes }) => Ok(*downloaded_bytes),
            Some(PieceStatus::MissingPiece) => Ok(0),
            _ => Err(TorrentStatusError::CalculatingBeginningByteIndex(
                "[MsgLogicControlError] Invalid piece index given in order to calculate beggining byte index."
                    .to_string(),
            )),
        }
    }

    /// Funcion que calcula la cantidad de bytes adecuada a pedir
    /// posteriormente a un peer
    ///
    pub fn calculate_amount_of_bytes_of_block(
        &self,
        torrent_file_data: &TorrentFileData,
        piece_index: usize,
        beginning_byte_index: u32,
    ) -> Result<u32, TorrentStatusError> {
        let piece_length = torrent_file_data
            .calculate_piece_lenght(piece_index)
            .map_err(|err| TorrentStatusError::CalculatingAmountOfBytes(format!("{:?}", err)))?;
        let piece_lenght = u32::try_from(piece_length)
            .map_err(|err| TorrentStatusError::CalculatingAmountOfBytes(format!("{:?}", err)))?;

        let remaining_bytes = piece_lenght - beginning_byte_index;
        if remaining_bytes <= BLOCK_BYTES {
            Ok(remaining_bytes)
        } else {
            Ok(BLOCK_BYTES)
        }
    }
}

#[cfg(test)]
mod test_torrent_status {
    mod test_look_for_a_missing_piece_index {
        use std::{
            error::Error,
            fmt,
            io::ErrorKind,
            net::{TcpListener, TcpStream},
            thread,
        };

        use crate::torrent::{
            data::{
                peer_data_for_communication::PeerDataForP2PCommunication,
                torrent_status::{StateOfDownload, TorrentStatus},
            },
            local_peer::{LocalPeer, PeerRole},
            parsers::p2p::message::PieceStatus,
        };

        const LOCALHOST: &str = "127.0.0.1";
        const STARTING_PORT: u16 = 8080;
        const MAX_TESTING_PORT: u16 = 9080;

        #[derive(PartialEq, Debug)]
        enum PortBindingError {
            ReachedMaxPortWithoutFindingAnAvailableOne,
        }

        impl fmt::Display for PortBindingError {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{:?}", self)
            }
        }

        impl Error for PortBindingError {}

        fn update_port(current_port: u16) -> Result<u16, PortBindingError> {
            let mut new_port: u16 = current_port;
            if current_port >= MAX_TESTING_PORT {
                Err(PortBindingError::ReachedMaxPortWithoutFindingAnAvailableOne)
            } else {
                new_port += 1;
                Ok(new_port)
            }
        }

        // Busca bindear un listener mientras que el error sea por causa de una direccion que ya está en uso.
        fn try_bind_listener(first_port: u16) -> Result<(TcpListener, String), Box<dyn Error>> {
            let mut listener = TcpListener::bind(format!("{}:{}", LOCALHOST, first_port));

            let mut current_port = first_port;

            while let Err(bind_err) = listener {
                if bind_err.kind() != ErrorKind::AddrInUse {
                    return Err(Box::new(bind_err));
                } else {
                    current_port = update_port(current_port)?;
                    listener = TcpListener::bind(format!("{}:{}", LOCALHOST, current_port));
                }
            }
            let resulting_listener = listener?; // SI BIEN TIENE ?; ACÁ NUNCA VA A SER UN ERROR

            Ok((
                resulting_listener,
                format!("{}:{}", LOCALHOST, current_port),
            ))
        }

        pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
        pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";

        //===============================================

        fn create_default_torrent_status_with_a_server_peer_that_has_the_whole_file(
        ) -> Result<(TorrentStatus, LocalPeer), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;

            let handler = thread::spawn(move || listener.accept());

            let stream = TcpStream::connect(address)?;
            handler.join().unwrap()?; // feo pero para probar

            let torrent_status = TorrentStatus {
                uploaded: 0,
                downloaded: 0,
                left: 40000,
                event: StateOfDownload::Started,
                pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
            };
            let server_peer_data = PeerDataForP2PCommunication {
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                pieces_availability: vec![
                    PieceStatus::ValidAndAvailablePiece,
                    PieceStatus::ValidAndAvailablePiece,
                ],
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };
            let local_peer = LocalPeer {
                peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
                external_peer_data: server_peer_data,
                role: PeerRole::Client,
                stream,
            };
            Ok((torrent_status, local_peer))
        }

        fn create_default_torrent_status_with_a_server_peer_that_has_just_one_valid_piece(
        ) -> Result<(TorrentStatus, LocalPeer), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;

            let handler = thread::spawn(move || listener.accept());

            let stream = TcpStream::connect(address)?;
            handler.join().unwrap()?; // feo pero para probar

            let torrent_status = TorrentStatus {
                uploaded: 0,
                downloaded: 0,
                left: 16,
                event: StateOfDownload::Started,
                pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
            };
            let server_peer_data = PeerDataForP2PCommunication {
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                pieces_availability: vec![
                    PieceStatus::MissingPiece,
                    PieceStatus::ValidAndAvailablePiece,
                ],
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };
            let local_peer = LocalPeer {
                peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
                external_peer_data: server_peer_data,
                role: PeerRole::Client,
                stream,
            };
            Ok((torrent_status, local_peer))
        }

        fn create_default_torrent_status_with_a_server_peer_that_has_no_valid_pieces(
        ) -> Result<(TorrentStatus, LocalPeer), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;

            let handler = thread::spawn(move || listener.accept());

            let stream = TcpStream::connect(address)?;
            handler.join().unwrap()?; // feo pero para probar

            let torrent_status = TorrentStatus {
                uploaded: 0,
                downloaded: 0,
                left: 16,
                event: StateOfDownload::Started,
                pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
            };
            let server_peer_data = PeerDataForP2PCommunication {
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };
            let local_peer = LocalPeer {
                peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
                external_peer_data: server_peer_data,
                role: PeerRole::Client,
                stream,
            };

            Ok((torrent_status, local_peer))
        }

        //===============================================

        #[test]
        fn the_server_peer_has_a_valid_and_available_piece_in_the_position_zero_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (torrent_status, local_peer) =
                create_default_torrent_status_with_a_server_peer_that_has_the_whole_file()?;

            assert_eq!(
                Some(0),
                torrent_status.look_for_a_missing_piece_index(&local_peer)
            );
            Ok(())
        }

        #[test]
        fn the_server_peer_has_a_valid_and_available_piece_in_the_position_one_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (torrent_status, local_peer) =
                create_default_torrent_status_with_a_server_peer_that_has_just_one_valid_piece()?;

            assert_eq!(
                Some(1),
                torrent_status.look_for_a_missing_piece_index(&local_peer)
            );
            Ok(())
        }

        #[test]
        fn the_server_peer_has_no_pieces_ok() -> Result<(), Box<dyn Error>> {
            let (torrent_status, local_peer) =
                create_default_torrent_status_with_a_server_peer_that_has_no_valid_pieces()?;

            assert_eq!(
                None,
                torrent_status.look_for_a_missing_piece_index(&local_peer)
            );
            Ok(())
        }

        #[test]
        fn the_server_peer_has_the_hole_file_and_the_client_peer_has_the_first_piece_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (mut torrent_status, local_peer) =
                create_default_torrent_status_with_a_server_peer_that_has_the_whole_file()?;
            torrent_status.pieces_availability[0] = PieceStatus::ValidAndAvailablePiece;

            assert_eq!(
                Some(1),
                torrent_status.look_for_a_missing_piece_index(&local_peer)
            );
            Ok(())
        }
    }
}
