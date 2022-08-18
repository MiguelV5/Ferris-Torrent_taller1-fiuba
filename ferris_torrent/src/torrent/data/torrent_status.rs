//! # Modulo de data actual de descarga
//! Este modulo contiene l&as funciones encargadas de almacenar y verificar
//! el estado actual de una descarga de un torrent
//!

use std::{error::Error, fmt};

use log::debug;

use crate::torrent::client::peers_communication::{
    handler_communication::BLOCK_BYTES, local_peer_communicator::LocalPeerCommunicator,
};

use shared::parsers::p2p::message::PieceStatus;

use super::torrent_file_data::TorrentFileData;

#[derive(PartialEq, Eq, Debug, Clone)]
/// Representa el estado de la descarga COMPLETA del torrent
pub enum TorrentStatusError {
    UpdatingPieceStatus(String),
    CalculatingBeginningByteIndex(String),
    CalculatingAmountOfBytes(String),
    CalculatingDownloadedPorcentage(String),
}

impl fmt::Display for TorrentStatusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for TorrentStatusError {}

#[derive(PartialEq, Eq, Debug, Clone)]
/// Representa el estado de la descarga COMPLETA del torrent
pub enum StateOfDownload {
    Started,
    Completed,
    Stopped,
}

#[derive(PartialEq, Eq, Debug, Clone)]
/// Representa la data actual de como va la descarga de un torrent
pub struct TorrentStatus {
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: StateOfDownload,
    pub pieces_availability: Vec<PieceStatus>,
}

fn is_valid_piece_to_request(piece_status: &PieceStatus) -> bool {
    matches!(
        *piece_status,
        PieceStatus::MissingPiece {
            was_requested: false,
        } | PieceStatus::PartiallyDownloaded {
            was_requested: false,
            ..
        }
    )
}

impl TorrentStatus {
    /// Funcion que crea informacion inicial del estado de descarga de un
    /// torrent.
    ///
    pub fn new(size_torrent: u64, total_amount_pieces: usize) -> Self {
        let mut pieces_availability = Vec::with_capacity(total_amount_pieces);
        pieces_availability.resize(
            total_amount_pieces,
            PieceStatus::MissingPiece {
                was_requested: false,
            },
        );

        TorrentStatus {
            uploaded: 0,
            downloaded: 0,
            left: size_torrent,
            event: StateOfDownload::Started,
            pieces_availability,
        }
    }

    pub fn get_downloaded_bytes(&self) -> u64 {
        self.downloaded
    }

    pub fn get_uploaded_bytes(&self) -> u64 {
        self.uploaded
    }

    pub fn get_porcentage_downloaded(&self) -> Result<f64, TorrentStatusError> {
        let left = self.left;
        let downloaded = u32::try_from(self.downloaded).map_err(|err| {
            TorrentStatusError::CalculatingDownloadedPorcentage(format!("{}", err))
        })?;
        let total = u32::try_from(left + self.downloaded).map_err(|err| {
            TorrentStatusError::CalculatingDownloadedPorcentage(format!("{}", err))
        })?;

        let downloaded = f64::from(downloaded);
        let total = f64::from(total);

        Ok(downloaded / total)
    }

    pub fn get_amount_of_downloaded_pieces(&self) -> u64 {
        self.pieces_availability
            .iter()
            .filter(|&piece_status| *piece_status == PieceStatus::ValidAndAvailablePiece)
            .count() as u64
    }

    /// Funcion que indica si una pieza de tal indice está faltante por descargar
    ///
    pub fn is_a_missing_piece(&self, piece_index: usize) -> bool {
        if let Some(piece_status) = self.pieces_availability.get(piece_index) {
            *piece_status
                == PieceStatus::MissingPiece {
                    was_requested: false,
                }
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

    pub fn get_piece_status(&self, piece_index: usize) -> Option<&PieceStatus> {
        self.pieces_availability.get(piece_index)
    }

    pub fn get_pieces_availability(&self) -> Vec<PieceStatus> {
        self.pieces_availability.clone()
    }

    pub fn increment_downloaded_counter(&mut self, amount_of_bytes: u64) {
        self.downloaded += amount_of_bytes;
        self.left -= amount_of_bytes;
    }

    pub fn increment_uploaded_counter(&mut self, amount_of_bytes: u64) {
        self.uploaded += amount_of_bytes;
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
            .map_err(|err| TorrentStatusError::UpdatingPieceStatus(format!("{}", err)))?;

        if let Some(piece_status) = self.pieces_availability.get_mut(piece_index) {
            match piece_status {
                PieceStatus::MissingPiece {
                    was_requested: true,
                } => {
                    if piece_lenght == u64::from(amount_of_bytes) {
                        *piece_status = PieceStatus::ValidAndAvailablePiece;
                    } else {
                        *piece_status = PieceStatus::PartiallyDownloaded {
                            downloaded_bytes: amount_of_bytes,
                            was_requested: false,
                        };
                    }
                    self.increment_downloaded_counter(amount_of_bytes.into())
                }
                PieceStatus::PartiallyDownloaded {
                    downloaded_bytes,
                    was_requested: true,
                } => {
                    let remaining_bytes =
                        piece_lenght - u64::from(*downloaded_bytes + amount_of_bytes);
                    if remaining_bytes == 0 {
                        *piece_status = PieceStatus::ValidAndAvailablePiece;
                    } else {
                        *piece_status = PieceStatus::PartiallyDownloaded {
                            downloaded_bytes: beginning_byte_index + amount_of_bytes,
                            was_requested: false,
                        };
                    }
                    self.increment_downloaded_counter(amount_of_bytes.into())
                }
                PieceStatus::ValidAndAvailablePiece => {
                    return Err(TorrentStatusError::UpdatingPieceStatus(
                        "[TorrentStatusError] The piece has already been completed.".to_string(),
                    ))
                }
                _ => {
                    return Err(TorrentStatusError::UpdatingPieceStatus(
                        "[TorrentStatusError] The received piece was not requested before."
                            .to_string(),
                    ))
                }
            }
        };
        debug!(
            "Nuevo estado de la pieza {}: {:?}",
            piece_index, self.pieces_availability[piece_index]
        );

        if self.all_pieces_completed() {
            self.event = StateOfDownload::Completed;
        }

        Ok(())
    }

    pub fn is_torrent_state_set_as_completed(&self) -> bool {
        self.event == StateOfDownload::Completed
    }

    /// Funcion que busca una nueva pieza que quiera pedir posteriormente, y
    /// devuelve su indice
    ///
    pub fn look_for_a_missing_piece_index(
        &self,
        local_peer: &LocalPeerCommunicator,
    ) -> Option<usize> {
        let (piece_index, _piece_status) =
            self.pieces_availability
                .iter()
                .enumerate()
                .find(|(piece_index, piece_status)| {
                    is_valid_piece_to_request(piece_status)
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
            Some(PieceStatus::PartiallyDownloaded { downloaded_bytes , was_requested: _ }) => Ok(*downloaded_bytes),
            Some(PieceStatus::MissingPiece{ was_requested: false}) => Ok(0),
            _ => Err(TorrentStatusError::CalculatingBeginningByteIndex(
                "[InteractionHandlerError] Invalid piece index given in order to calculate beggining byte index."
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
            .map_err(|err| TorrentStatusError::CalculatingAmountOfBytes(format!("{}", err)))?;
        let piece_lenght = u32::try_from(piece_length)
            .map_err(|err| TorrentStatusError::CalculatingAmountOfBytes(format!("{}", err)))?;

        let remaining_bytes = piece_lenght - beginning_byte_index;
        if remaining_bytes <= BLOCK_BYTES {
            Ok(remaining_bytes)
        } else {
            Ok(BLOCK_BYTES)
        }
    }

    pub fn all_pieces_left(&self) -> bool {
        self.pieces_availability
            .iter()
            .any(|piece_status| *piece_status == PieceStatus::ValidAndAvailablePiece)
    }

    pub fn all_pieces_completed(&self) -> bool {
        self.pieces_availability
            .iter()
            .all(|piece| *piece == PieceStatus::ValidAndAvailablePiece)
    }

    pub fn set_piece_as_requested(&mut self, piece_index: usize) -> Result<(), TorrentStatusError> {
        if let Some(piece_status) = self.pieces_availability.get_mut(piece_index) {
            match piece_status {
                PieceStatus::MissingPiece { was_requested } => {
                    *was_requested = true;
                    Ok(())
                }
                PieceStatus::PartiallyDownloaded { was_requested, .. } => {
                    *was_requested = true;
                    Ok(())
                }
                _ =>  Err(TorrentStatusError::UpdatingPieceStatus("[TorrentStatusError] A valid and available piece cannot be setting as requested.".to_string())),
            }
        } else {
            Err(TorrentStatusError::UpdatingPieceStatus(
                "[TorrentStatusError] The given piece index does not match with a piece."
                    .to_string(),
            ))
        }
    }

    pub fn set_all_pieces_as_not_requested(&mut self) {
        for piece_status in self.pieces_availability.iter_mut() {
            match piece_status {
                PieceStatus::MissingPiece { was_requested } => {
                    *was_requested = false;
                }
                PieceStatus::PartiallyDownloaded { was_requested, .. } => {
                    *was_requested = false;
                }
                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod test_torrent_status {
    mod test_look_for_a_missing_piece_index {
        use std::{error::Error, net::TcpStream, sync::mpsc, thread, time::SystemTime};

        use gtk::glib;

        use crate::torrent::client::peers_communication::local_peer_communicator::{
            LocalPeerCommunicator, PeerRole,
        };
        use crate::torrent::{
            data::{
                peer_data_for_communication::PeerDataForP2PCommunication,
                torrent_status::{StateOfDownload, TorrentStatus},
            },
            port_testing::listener_binder::*,
        };
        use shared::parsers::p2p::message::PieceStatus;

        pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
        pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";

        //===============================================

        fn create_default_torrent_status_with_a_server_peer_that_has_the_whole_file(
        ) -> Result<(TorrentStatus, LocalPeerCommunicator), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;

            let handler = thread::spawn(move || listener.accept());

            let stream = TcpStream::connect(address)?;
            let _joined = handler.join();

            let torrent_status = TorrentStatus {
                uploaded: 0,
                downloaded: 0,
                left: 40000,
                event: StateOfDownload::Started,
                pieces_availability: vec![
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                ],
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
            let (ui_sender, _) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let local_peer = LocalPeerCommunicator {
                peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
                stream,
                external_peer_data: server_peer_data,
                role: PeerRole::Client,
                logger_sender: mpsc::channel().0,
                ui_sender: ui_sender,
                clock: SystemTime::now(),
            };
            Ok((torrent_status, local_peer))
        }

        fn create_default_torrent_status_with_a_server_peer_that_has_just_one_valid_piece(
        ) -> Result<(TorrentStatus, LocalPeerCommunicator), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;

            let handler = thread::spawn(move || listener.accept());

            let stream = TcpStream::connect(address)?;
            let _joined = handler.join();

            let torrent_status = TorrentStatus {
                uploaded: 0,
                downloaded: 0,
                left: 16,
                event: StateOfDownload::Started,
                pieces_availability: vec![
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                ],
            };
            let server_peer_data = PeerDataForP2PCommunication {
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                pieces_availability: vec![
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                    PieceStatus::ValidAndAvailablePiece,
                ],
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };
            let (ui_sender, _) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let local_peer = LocalPeerCommunicator {
                peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
                stream,
                external_peer_data: server_peer_data,
                role: PeerRole::Client,
                logger_sender: mpsc::channel().0,
                ui_sender: ui_sender,
                clock: SystemTime::now(),
            };
            Ok((torrent_status, local_peer))
        }

        fn create_default_torrent_status_with_a_server_peer_that_has_no_valid_pieces(
        ) -> Result<(TorrentStatus, LocalPeerCommunicator), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;

            let handler = thread::spawn(move || listener.accept());

            let stream = TcpStream::connect(address)?;
            let _joined = handler.join();

            let torrent_status = TorrentStatus {
                uploaded: 0,
                downloaded: 0,
                left: 16,
                event: StateOfDownload::Started,
                pieces_availability: vec![
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                ],
            };
            let server_peer_data = PeerDataForP2PCommunication {
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                pieces_availability: vec![
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                ],
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };
            let (ui_sender, _) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
            let local_peer = LocalPeerCommunicator {
                peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
                stream,
                external_peer_data: server_peer_data,
                role: PeerRole::Client,
                logger_sender: mpsc::channel().0,
                ui_sender: ui_sender,
                clock: SystemTime::now(),
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
        fn the_server_peer_has_the_whole_file_and_the_client_peer_has_the_first_piece_ok(
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
