//! # Modulo de manejo general de la estructura principal: Client
//! Este modulo contiene las funciones encargadas del comportamiento general
//! de nuestro cliente como peer de tipo leecher.

use crate::torrent::{
    client::{
        block_handler,
        peers_communication::{msg_receiver, msg_sender},
    },
    data::{
        peer_data_for_communication::PeerDataForP2PCommunication,
        torrent_file_data::TorrentFileData, torrent_status::TorrentStatus,
        tracker_response_data::TrackerResponseData,
    },
    parsers::p2p::{
        constants::PSTR_STRING_HANDSHAKE,
        message::{P2PMessage, PieceStatus},
    },
    user_interface::{constants::MessageUI, ui_sender_handler},
};
extern crate rand;
use gtk::glib::Sender as UiSender;
use log::{debug, info};
use rand::{distributions::Alphanumeric, Rng};
use std::{
    error::Error,
    fmt,
    net::{SocketAddr, TcpStream},
    sync::{Arc, RwLock},
    time::Duration,
};
use std::{sync::mpsc::Sender as LoggerSender, time::SystemTime};

//========================================================

const SIZE_PEER_ID: usize = 12;
const INIT_PEER_ID: &str = "-FA0000-";

pub const SECS_READ_TIMEOUT: u64 = 10; //120 debe estar
pub const SECS_CONNECT_TIMEOUT: u64 = 10;

//========================================================

#[derive(Debug)]
/// Struct que tiene por comportamiento todo el manejo general de actualizacion importante de datos, almacenamiento de los mismos y ejecución de metodos importantes para la comunicación con peers durante la ejecución del programa a modo de leecher.
pub struct LocalPeerCommunicator {
    pub peer_id: Vec<u8>,
    pub stream: TcpStream,
    pub external_peer_data: PeerDataForP2PCommunication,
    pub role: PeerRole,
    pub logger_sender: LoggerSender<String>,
    pub ui_sender: UiSender<MessageUI>,
    pub clock: SystemTime,
}

#[derive(PartialEq, Debug, Clone)]
pub enum PeerRole {
    Client,
    Server,
}

//========================================================

//Ver si corresponde este enum en otro lugar
/// Enum para distincion de almacenamiento al recibir un mensaje Piece
enum InterestOfReceivedPieceMsg {
    AlreadyDownloaded,
    IsCorrectlyAsRequested,
}

//========================================================

#[derive(PartialEq, Debug, Clone)]
pub enum InteractionHandlerErrorKind {
    Recoverable(InteractionHandlerError),
    Unrecoverable(InteractionHandlerError),
}

impl fmt::Display for InteractionHandlerErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for InteractionHandlerErrorKind {}

#[derive(PartialEq, Debug, Clone)]
/// Representa un tipo de error en la comunicación general P2P con un peer individual.
pub enum InteractionHandlerError {
    ConectingWithPeer(String),
    RestartingDownload(String),
    UpdatingBitfield(String),
    LookingForPieces(String),
    CheckingAndSavingHandshake(String),
    ReceivingHanshake(String),
    ReceivingMessage(String),
    SendingHandshake(String),
    SendingMessage(String),
    UpdatingPieceStatus(String),
    StoringBlock(String),
    CalculatingServerPeerIndex(String),
    CalculatingPieceLenght(String),
    SetUpDirectory(String),
    SendingRequestedBlock(String),
    LockingTorrentStatus(String),
    JoinHandle(String),
    WritingShutDownField(String),

    ReadingShutDownField(String),
    UpdatingWasRequestedField(String),
    LogError(String),
    UiError(String),
    CalculatingTime(String),
    PiecesHandler(String),
}

impl fmt::Display for InteractionHandlerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for InteractionHandlerError {}

#[derive(PartialEq, Debug, Clone)]
/// Representa un tipo de estado de interaccion para saber si se debe
/// continuar o finalizar la misma
pub enum InteractionHandlerStatus {
    LookForAnotherPeer,
    FinishInteraction,
    SecureLocalShutDown,
    SecureGlobalShutDown,
}

//========================================================

/// Funcion que crea un peer id unico para este cliente como peer
///
pub fn generate_peer_id() -> Vec<u8> {
    let rand_alphanumeric: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(SIZE_PEER_ID)
        .map(char::from)
        .collect();
    let str_peer = format!("{}{}", INIT_PEER_ID, rand_alphanumeric);
    debug!("Peer_id: {}", str_peer);
    str_peer.as_bytes().to_vec()
}

fn open_connection_with_peer(
    tracker_response: &TrackerResponseData,
    tracker_response_peer_index: usize,
) -> Result<(TcpStream, SocketAddr), InteractionHandlerErrorKind> {
    if let Some(external_peer_address) =
        tracker_response.get_peer_address(tracker_response_peer_index)
    {
        let stream = TcpStream::connect_timeout(
            &external_peer_address,
            Duration::from_secs(SECS_READ_TIMEOUT),
        )
        .map_err(|error| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::ConectingWithPeer(
                format!("{}", error),
            ))
        })?;

        stream
            .set_read_timeout(Some(Duration::from_secs(SECS_READ_TIMEOUT)))
            .map_err(|err| {
                InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::ConectingWithPeer(format!("{}", err)),
                )
            })?;
        return Ok((stream, external_peer_address));
    }

    Err(InteractionHandlerErrorKind::Unrecoverable(
        InteractionHandlerError::ConectingWithPeer(String::from(
            "[InteractionHandlerError] Peer doesn't have an address stored in the tracker response.",
        )),
    ))
}

//HANDSHAKE
fn check_handshake(
    torrent_file_data: &TorrentFileData,
    server_protocol_str: String,
    server_info_hash: &[u8],
) -> Result<(), InteractionHandlerErrorKind> {
    if (server_protocol_str == PSTR_STRING_HANDSHAKE)
        && torrent_file_data.has_expected_info_hash(server_info_hash)
    {
        Ok(())
    } else {
        Err(InteractionHandlerErrorKind::Recoverable(
            InteractionHandlerError::CheckingAndSavingHandshake(
                "[InteractionHandlerError] The received handshake hasn`t got the expected fields."
                    .to_string(),
            ),
        ))
    }
}

fn check_peer_id(
    tracker_response: &TrackerResponseData,
    tracker_response_peer_index: usize,
    server_peer_id: &[u8],
) -> Result<(), InteractionHandlerErrorKind> {
    if tracker_response.has_expected_peer_id(tracker_response_peer_index, server_peer_id) {
        Ok(())
    } else {
        Err(InteractionHandlerErrorKind::Recoverable(
            InteractionHandlerError::CheckingAndSavingHandshake(
                "[InteractionHandlerError] The received handshake hasn`t got the expected fields."
                    .to_string(),
            ),
        ))
    }
}

/// Funcion que realiza la verificacion de un mensaje recibido de tipo
/// Handshake y almacena su info importante
///
fn generate_peer_data_from_handshake_torrent_peer(
    message: P2PMessage,
    torrent_file_data: &TorrentFileData,
    tracker_response: &TrackerResponseData,
    tracker_response_peer_index: usize,
) -> Result<PeerDataForP2PCommunication, InteractionHandlerErrorKind> {
    if let P2PMessage::Handshake {
        protocol_str: server_protocol_str,
        info_hash: server_info_hash,
        peer_id: server_peer_id,
    } = message
    {
        check_handshake(torrent_file_data, server_protocol_str, &server_info_hash)?;
        check_peer_id(
            tracker_response,
            tracker_response_peer_index,
            &server_peer_id,
        )?;
        Ok(PeerDataForP2PCommunication::new(
            torrent_file_data,
            server_peer_id,
        ))
    } else {
        Err(InteractionHandlerErrorKind::Recoverable(
            InteractionHandlerError::CheckingAndSavingHandshake(
                "[InteractionHandlerError] The received messagge is not a handshake.".to_string(),
            ),
        ))
    }
}

fn generate_peer_data_from_handshake_new_peer(
    message: P2PMessage,
    torrent_file_data: &TorrentFileData,
) -> Result<PeerDataForP2PCommunication, InteractionHandlerErrorKind> {
    if let P2PMessage::Handshake {
        protocol_str: server_protocol_str,
        info_hash: server_info_hash,
        peer_id: server_peer_id,
    } = message
    {
        check_handshake(torrent_file_data, server_protocol_str, &server_info_hash)?;
        Ok(PeerDataForP2PCommunication::new(
            torrent_file_data,
            server_peer_id,
        ))
    } else {
        Err(InteractionHandlerErrorKind::Recoverable(
            InteractionHandlerError::CheckingAndSavingHandshake(
                "[InteractionHandlerError] The received messagge is not a handshake.".to_string(),
            ),
        ))
    }
}

fn log_info_msg(msg: &P2PMessage) {
    match &msg {
        P2PMessage::Piece {
            piece_index,
            beginning_byte_index,
            block: _,
        } => info!(
            "Mensaje recibido: Piece[piece_index: {}, beginning_byte_index: {}]",
            piece_index, beginning_byte_index
        ),
        P2PMessage::Bitfield { bitfield: _ } => info!("Mensaje recibido: Bitfield"),
        _ => info!("Mensaje recibido: {:?}", msg),
    }
}

fn is_local_shut_down_set(
    local_shut_down: &Arc<RwLock<bool>>,
) -> Result<bool, InteractionHandlerErrorKind> {
    let local_shut_down = local_shut_down.read().map_err(|error| {
        InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::ReadingShutDownField(
            format!("{:?}", error),
        ))
    })?;
    Ok(*local_shut_down)
}

fn is_global_shut_down_set(
    global_shut_down: &Arc<RwLock<bool>>,
) -> Result<bool, InteractionHandlerErrorKind> {
    let global_shut_down = global_shut_down.read().map_err(|error| {
        InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::ReadingShutDownField(
            format!("{:?}", error),
        ))
    })?;
    Ok(*global_shut_down)
}

// --------------------------------------------------

impl LocalPeerCommunicator {
    // FUNCIONES PRINCIPALES

    ///
    ///
    pub fn start_communication_as_client(
        torrent_file_data: &TorrentFileData,
        tracker_response: &TrackerResponseData,
        tracker_response_peer_index: usize,
        peer_id: Vec<u8>,
        logger_sender: LoggerSender<String>,
        ui_sender: UiSender<MessageUI>,
    ) -> Result<Self, InteractionHandlerErrorKind> {
        let (mut local_peer_stream, external_peer_addr) =
            open_connection_with_peer(tracker_response, tracker_response_peer_index)?;
        info!("El cliente se conecta con un peer exitosamente.");

        msg_sender::send_handshake(&mut local_peer_stream, &peer_id, torrent_file_data).map_err(
            |error| {
                InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::SendingHandshake(
                    format!("{}", error),
                ))
            },
        )?;
        info!("Mensaje enviado: Handshake.");

        let received_handshake =
            msg_receiver::receive_handshake(&mut local_peer_stream).map_err(|error| {
                InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::ReceivingHanshake(format!("{}", error)),
                )
            })?;
        info!("Mensaje recibido: Handshake.");

        let external_peer_data = generate_peer_data_from_handshake_torrent_peer(
            received_handshake,
            torrent_file_data,
            tracker_response,
            tracker_response_peer_index,
        )?;

        let time = SystemTime::now();

        ui_sender_handler::add_external_peer(
            &ui_sender,
            torrent_file_data,
            &external_peer_data,
            &external_peer_addr,
        )
        .map_err(|error| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UiError(format!(
                "{}",
                error
            )))
        })?;

        Ok(LocalPeerCommunicator {
            peer_id,
            stream: local_peer_stream,
            external_peer_data,
            role: PeerRole::Client,
            logger_sender,
            ui_sender,
            clock: time,
        })
    }

    ///
    ///
    pub fn start_communication_as_server(
        torrent_file_data: &TorrentFileData,
        peer_id: Vec<u8>,
        mut stream: TcpStream,
        external_peer_addr: SocketAddr,
        logger_sender: LoggerSender<String>,
        ui_sender: UiSender<MessageUI>,
    ) -> Result<Self, InteractionHandlerErrorKind> {
        let received_handshake = msg_receiver::receive_handshake(&mut stream).map_err(|error| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::ReceivingHanshake(
                format!("{}", error),
            ))
        })?;
        info!("Mensaje recibido: Handshake.");

        let external_peer_data =
            generate_peer_data_from_handshake_new_peer(received_handshake, torrent_file_data)?;

        msg_sender::send_handshake(&mut stream, &peer_id, torrent_file_data).map_err(|error| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::SendingHandshake(
                format!("{}", error),
            ))
        })?;
        info!("Mensaje enviado: Handshake.");

        let time = SystemTime::now();

        ui_sender_handler::add_external_peer(
            &ui_sender,
            torrent_file_data,
            &external_peer_data,
            &external_peer_addr,
        )
        .map_err(|error| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UiError(format!(
                "{}",
                error
            )))
        })?;

        Ok(LocalPeerCommunicator {
            peer_id,
            stream,
            external_peer_data,
            role: PeerRole::Client,
            logger_sender,
            ui_sender,
            clock: time,
        })
    }

    ///
    ///
    pub fn interact_with_peer(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &Arc<RwLock<TorrentStatus>>,
        global_shut_down: &Arc<RwLock<bool>>,
        local_shut_down: &Arc<RwLock<bool>>,
    ) -> Result<InteractionHandlerStatus, InteractionHandlerErrorKind> {
        self.send_bitfield_if_necessary(torrent_status)?;

        loop {
            //esto es lo importanet de la funcion que ya quedó modularizado bien
            let received_msg =
                msg_receiver::receive_message(&mut self.stream).map_err(|error| {
                    InteractionHandlerErrorKind::Recoverable(
                        InteractionHandlerError::ReceivingMessage(format!("{:?}", error)),
                    )
                })?;
            log_info_msg(&received_msg);

            self.update_information_according_to_the_received_msg(
                torrent_file_data,
                torrent_status,
                &received_msg,
            )?;

            self.react_according_to_the_peer_role(
                torrent_file_data,
                torrent_status,
                &received_msg,
            )?;
            //------

            //todo esto es la condicion de corte que bien podria ir afuera capaz o modularizado
            if is_local_shut_down_set(local_shut_down)? {
                return Ok(InteractionHandlerStatus::SecureLocalShutDown);
            } else if is_global_shut_down_set(global_shut_down)? {
                return Ok(InteractionHandlerStatus::SecureGlobalShutDown);
            }
            //esto deberia tener otro error y capaza se puede sacar del loop
            let torrent_status = torrent_status.read().map_err(|error| {
                InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::SendingMessage(
                    format!("{:?}", error),
                ))
            })?;
            if torrent_status.is_torrent_state_set_as_completed() && !self.peer_interested() {
                return Ok(InteractionHandlerStatus::FinishInteraction);
            } else if !self.am_interested() && !self.peer_interested() {
                info!("Se busca un nuevo peer al cual pedirle piezas");
                return Ok(InteractionHandlerStatus::LookForAnotherPeer);
            }
        }
    }

    //FUNCIONES SECUNDARIAS

    fn send_bitfield_if_necessary(
        &mut self,
        torrent_status: &Arc<RwLock<TorrentStatus>>,
    ) -> Result<(), InteractionHandlerErrorKind> {
        let torrent_status = torrent_status.read().map_err(|error| {
            InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::SendingMessage(
                format!("{:?}", error),
            ))
        })?;

        if torrent_status.all_pieces_left() {
            msg_sender::send_bitfield(&mut self.stream, &torrent_status).map_err(|error| {
                InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::SendingMessage(
                    format!("{:?}", error),
                ))
            })?;
        }
        Ok(())
    }

    //BITFIELD
    /// Funcion que actualiza la representación de bitfield de un peer dado
    /// por su indice
    ///
    fn update_peer_bitfield(
        &mut self,
        torrent_file_data: &TorrentFileData,
        bitfield: &[PieceStatus],
    ) -> Result<(), InteractionHandlerErrorKind> {
        let mut pieces_availability = bitfield.to_vec();
        torrent_file_data
            .check_bitfield(&pieces_availability)
            .map_err(|err| {
                InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UpdatingBitfield(
                    format!("{}", err),
                ))
            })?;
        pieces_availability.truncate(torrent_file_data.get_total_amount_pieces());
        self.external_peer_data
            .update_pieces_availability(pieces_availability);
        Ok(())
    }

    // HAVE
    /// Funcion que actualiza la representación de bitfield de un peer dado
    /// por su indice (A diferencia de [update_peer_bitfield()], esta funcion
    /// actualiza solo el estado de UNA pieza, esto es causado por
    /// la recepcion de un mensaje P2P de tipo Have)
    ///
    fn update_server_peer_piece_status(
        &mut self,
        piece_index: usize,
        new_status: PieceStatus,
    ) -> Result<(), InteractionHandlerErrorKind> {
        self.external_peer_data
            .update_piece_status(piece_index, new_status)
            .map_err(|err| {
                InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::UpdatingPieceStatus(format!("{}", err)),
                )
            })?;
        Ok(())
    }

    //PIECE
    fn check_piece_index_and_beginning_byte_index(
        &self,
        torrent_status: &TorrentStatus,
        piece_index: usize,
        beginning_byte_index: u32,
    ) -> Result<InterestOfReceivedPieceMsg, InteractionHandlerErrorKind> {
        match torrent_status.get_piece_status(piece_index) {
            Some(piece_status) => match *piece_status {
                PieceStatus::MissingPiece {
                    was_requested: true,
                } => Ok(InterestOfReceivedPieceMsg::IsCorrectlyAsRequested),
                PieceStatus::MissingPiece {
                    was_requested: false,
                } => Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::StoringBlock(
                        "[InteractionHandlerError] The received piece was not requested."
                            .to_string(),
                    ),
                )),
                PieceStatus::PartiallyDownloaded {
                    was_requested: false,
                    ..
                } => Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::StoringBlock(
                        "[InteractionHandlerError] The received piece was not requested."
                            .to_string(),
                    ),
                )),
                PieceStatus::PartiallyDownloaded {
                    downloaded_bytes,
                    was_requested: true,
                } if beginning_byte_index > downloaded_bytes => {
                    Err(InteractionHandlerErrorKind::Recoverable(
                        InteractionHandlerError::StoringBlock(
                            "[InteractionHandlerError] The beginning byte index is incorrect."
                                .to_string(),
                        ),
                    ))
                }
                PieceStatus::PartiallyDownloaded {
                    downloaded_bytes,
                    was_requested: true,
                } if beginning_byte_index < downloaded_bytes => {
                    Ok(InterestOfReceivedPieceMsg::AlreadyDownloaded)
                }
                PieceStatus::ValidAndAvailablePiece => {
                    Ok(InterestOfReceivedPieceMsg::AlreadyDownloaded)
                }
                _ => Ok(InterestOfReceivedPieceMsg::IsCorrectlyAsRequested),
            },
            None => Err(InteractionHandlerErrorKind::Recoverable(
                InteractionHandlerError::StoringBlock(
                    "[InteractionHandlerError] The received piece index is invalid.".to_string(),
                ),
            )),
        }
    }

    fn check_block_lenght(
        &self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &TorrentStatus,
        piece_index: usize,
        beginning_byte_index: u32,
        block: &[u8],
    ) -> Result<InterestOfReceivedPieceMsg, InteractionHandlerErrorKind> {
        let expected_amount_of_bytes = torrent_status
            .calculate_amount_of_bytes_of_block(
                torrent_file_data,
                piece_index,
                beginning_byte_index,
            )
            .map_err(|err| {
                InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::StoringBlock(
                    format!("{}", err),
                ))
            })?
            .try_into()
            .map_err(|err| {
                InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::StoringBlock(
                    format!("{}", err),
                ))
            })?;
        if block.len() != expected_amount_of_bytes {
            return Err(InteractionHandlerErrorKind::Recoverable(
                InteractionHandlerError::StoringBlock(
                    "[InteractionHandlerError] Block length is not as expected".to_string(),
                ),
            ));
        }
        Ok(InterestOfReceivedPieceMsg::IsCorrectlyAsRequested)
    }

    fn check_store_block(
        &self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &TorrentStatus,
        piece_index: usize,
        beginning_byte_index: u32,
        block: &[u8],
    ) -> Result<InterestOfReceivedPieceMsg, InteractionHandlerErrorKind> {
        match self.check_piece_index_and_beginning_byte_index(
            torrent_status,
            piece_index,
            beginning_byte_index,
        ) {
            Ok(InterestOfReceivedPieceMsg::IsCorrectlyAsRequested) => (),
            Ok(InterestOfReceivedPieceMsg::AlreadyDownloaded) => {
                return Ok(InterestOfReceivedPieceMsg::AlreadyDownloaded)
            }
            Err(error) => return Err(error),
        }
        self.check_block_lenght(
            torrent_file_data,
            torrent_status,
            piece_index,
            beginning_byte_index,
            block,
        )?;
        Ok(InterestOfReceivedPieceMsg::IsCorrectlyAsRequested)
    }

    fn check_piece(
        &self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &mut TorrentStatus,
        path: &str,
        piece_index: usize,
    ) -> Result<(), InteractionHandlerErrorKind> {
        if torrent_status.is_a_valid_and_available_piece(piece_index) {
            info!("Se completó la pieza {}.", piece_index);
            info!("Verifico el hash SHA1 de la pieza descargada.");
            block_handler::check_sha1_piece(torrent_file_data, piece_index, path).map_err(
                |err| {
                    InteractionHandlerErrorKind::Unrecoverable(
                        InteractionHandlerError::StoringBlock(format!("{}", err)),
                    )
                },
            )?;
            self.logger_sender
                .send(format!("[OK] Se completó la pieza número {}.", piece_index))
                .map_err(|err| {
                    InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::LogError(
                        format!("{}", err),
                    ))
                })?;
            ui_sender_handler::update_torrent_status(
                &self.ui_sender,
                torrent_file_data,
                torrent_status,
            )
            .map_err(|error| {
                InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UiError(format!(
                    "{}",
                    error
                )))
            })?;
        }
        Ok(())
    }

    /// Funcion encargada de realizar toda la logica de guardado de
    /// un bloque en disco y actualizacion correspondiente de
    /// mi propio bitfield y el estado de la descarga.
    /// Si se completa una pieza tras el guardado, se verifica la
    /// misma por medio de su SHA1 y el que venia como correspondiente
    /// a dicha pieza en el .torrent
    ///
    fn store_block(
        &self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &Arc<RwLock<TorrentStatus>>,
        path: &str,
        piece_index: usize,
        beginning_byte_index: u32,
        block: Vec<u8>,
    ) -> Result<(), InteractionHandlerErrorKind> {
        let mut torrent_status = torrent_status.write().map_err(|err| {
            InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::StoringBlock(
                format!("{}", err),
            ))
        })?;
        match self.check_store_block(
            torrent_file_data,
            &torrent_status,
            piece_index,
            beginning_byte_index,
            &block,
        ) {
            Ok(InterestOfReceivedPieceMsg::AlreadyDownloaded) => return Ok(()),
            Ok(InterestOfReceivedPieceMsg::IsCorrectlyAsRequested) => (),
            Err(error) => return Err(error),
        };
        block_handler::store_block(&block, piece_index, path).map_err(|err| {
            InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::StoringBlock(
                format!("{}", err),
            ))
        })?;

        torrent_status
            .update_piece_status(
                torrent_file_data,
                piece_index,
                beginning_byte_index,
                u32::try_from(block.len()).map_err(|err| {
                    InteractionHandlerErrorKind::Unrecoverable(
                        InteractionHandlerError::StoringBlock(format!("{}", err)),
                    )
                })?,
            )
            .map_err(|err| {
                InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::StoringBlock(
                    format!("{}", err),
                ))
            })?;

        self.check_piece(torrent_file_data, &mut torrent_status, path, piece_index)?;

        let download_duration = self.clock.elapsed().map_err(|err| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::CalculatingTime(
                format!("{}", err),
            ))
        })?;
        ui_sender_handler::update_download_data(
            &self.ui_sender,
            torrent_file_data,
            &self.external_peer_data,
            torrent_status.get_downloaded_bytes(),
            download_duration,
        )
        .map_err(|err| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UiError(format!(
                "{}",
                err
            )))
        })?;
        Ok(())
    }

    // UPDATING FIELDS

    /// Funcion que actualiza si el cliente está interesado en una pieza
    /// de un peer dado por su indice.
    ///
    fn update_am_interested_field(
        &mut self,
        torrent_file_data: &TorrentFileData,

        new_value: bool,
    ) -> Result<(), InteractionHandlerErrorKind> {
        self.external_peer_data.am_interested = new_value;
        ui_sender_handler::update_peers_state(
            &self.ui_sender,
            torrent_file_data,
            &self.external_peer_data,
        )
        .map_err(|error| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UiError(format!(
                "{}",
                error
            )))
        })?;
        Ok(())
    }

    /// Funcion que actualiza si un peer me tiene chokeado a mi cliente
    ///
    fn update_peer_choking_field(
        &mut self,
        torrent_file_data: &TorrentFileData,

        new_value: bool,
    ) -> Result<(), InteractionHandlerErrorKind> {
        self.external_peer_data.peer_choking = new_value;
        ui_sender_handler::update_peers_state(
            &self.ui_sender,
            torrent_file_data,
            &self.external_peer_data,
        )
        .map_err(|error| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UiError(format!(
                "{}",
                error
            )))
        })?;
        Ok(())
    }

    /// Funcion que actualiza si un peer está interesado en alguna de nuestras piezas
    ///
    fn update_peer_interested_field(
        &mut self,
        torrent_file_data: &TorrentFileData,

        new_value: bool,
    ) -> Result<(), InteractionHandlerErrorKind> {
        self.external_peer_data.peer_interested = new_value;
        ui_sender_handler::update_peers_state(
            &self.ui_sender,
            torrent_file_data,
            &self.external_peer_data,
        )
        .map_err(|error| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UiError(format!(
                "{}",
                error
            )))
        })?;
        Ok(())
    }

    fn set_up_peer_roll_as_client(&mut self) -> Result<(), InteractionHandlerErrorKind> {
        self.role = PeerRole::Client;
        Ok(())
    }

    fn set_up_peer_roll_as_server(&mut self) -> Result<(), InteractionHandlerErrorKind> {
        self.role = PeerRole::Server;
        Ok(())
    }

    // ASK FOR INFORMATION

    /// Funcion que revisa si el local peer tiene chokeado al external peer
    ///
    pub fn am_choking(&self) -> bool {
        self.external_peer_data.am_choking
    }

    /// Funcion que revisa si el local peer está interesado en alguna de las piezas del external peer
    ///
    pub fn am_interested(&self) -> bool {
        self.external_peer_data.am_interested
    }

    /// Funcion que revisa si el external peer tiene chokeado al local peer
    ///
    pub fn peer_choking(&self) -> bool {
        self.external_peer_data.peer_choking
    }

    /// Funcion que revisa si el external peer está interesado en alguna de las piezas del local peer
    ///
    pub fn peer_interested(&self) -> bool {
        self.external_peer_data.peer_interested
    }

    pub fn get_peer_id(&self) -> Vec<u8> {
        self.peer_id.clone()
    }

    pub fn external_peer_has_a_valid_and_available_piece_on_position(
        &self,
        position: usize,
    ) -> bool {
        self.external_peer_data.pieces_availability[position] == PieceStatus::ValidAndAvailablePiece
    }

    fn react_to_received_piece_msg(
        &mut self,
        piece_index: u32,
        beginning_byte_index: u32,
        block: &[u8],
        torrent_file_data: &TorrentFileData,
        torrent_status: &Arc<RwLock<TorrentStatus>>,
    ) -> Result<(), InteractionHandlerErrorKind> {
        let block = block.to_vec();
        let temp_path_name = torrent_file_data.get_torrent_representative_name();
        let piece_index = piece_index.try_into().map_err(|err| {
            InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::StoringBlock(
                format!("{}", err),
            ))
        })?;
        self.store_block(
            torrent_file_data,
            torrent_status,
            &temp_path_name,
            piece_index,
            beginning_byte_index,
            block,
        )?;

        Ok(())
    }

    //UPDATE INFORMATION
    fn update_information_according_to_the_received_msg(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &Arc<RwLock<TorrentStatus>>,
        received_msg: &P2PMessage,
    ) -> Result<(), InteractionHandlerErrorKind> {
        match received_msg {
            P2PMessage::KeepAlive => Ok(()),
            P2PMessage::Choke => self.update_peer_choking_field(torrent_file_data, true),
            P2PMessage::Unchoke => self.update_peer_choking_field(torrent_file_data, false),
            P2PMessage::Interested => {
                self.update_peer_interested_field(torrent_file_data, true)?;
                self.set_up_peer_roll_as_server()
            }
            P2PMessage::NotInterested => {
                self.update_peer_interested_field(torrent_file_data, false)
            }
            P2PMessage::Have { piece_index } => self.update_server_peer_piece_status(
                usize::try_from(*piece_index).map_err(|err| {
                    InteractionHandlerErrorKind::Unrecoverable(
                        InteractionHandlerError::LookingForPieces(format!("{}", err)),
                    )
                })?,
                PieceStatus::ValidAndAvailablePiece,
            ),
            P2PMessage::Bitfield { bitfield } => {
                self.update_peer_bitfield(torrent_file_data, bitfield)
            }
            P2PMessage::Request { .. } => self.set_up_peer_roll_as_server(),
            P2PMessage::Piece {
                piece_index,
                beginning_byte_index,
                block,
            } => {
                self.react_to_received_piece_msg(
                    *piece_index,
                    *beginning_byte_index,
                    block,
                    torrent_file_data,
                    torrent_status,
                )?;
                Ok(())
            }
            _ => Ok(()),
        }?;
        Ok(())
    }

    //LOOK FOR PIECES AND SEND MESSAGE
    fn send_msg_according_to_peer_choking_field(
        &mut self,
        piece_index: usize,
        torrent_file_data: &TorrentFileData,
        torrent_status: &mut TorrentStatus,
    ) -> Result<(), InteractionHandlerErrorKind> {
        if self.peer_choking() {
            info!("Mensaje enviado: Interested");
            msg_sender::send_interested(&mut self.stream).map_err(|err| {
                InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::SendingMessage(
                    format!("{}", err),
                ))
            })?;
        } else {
            msg_sender::send_request(
                &mut self.stream,
                torrent_file_data,
                torrent_status,
                piece_index,
            )
            .map_err(|err| {
                InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::SendingMessage(
                    format!("{}", err),
                ))
            })?;
            torrent_status
                .set_piece_as_requested(piece_index)
                .map_err(|err| {
                    InteractionHandlerErrorKind::Unrecoverable(
                        InteractionHandlerError::SendingMessage(format!("{}", err)),
                    )
                })?;
        }

        Ok(())
    }

    fn look_for_pieces(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &Arc<RwLock<TorrentStatus>>,
    ) -> Result<(), InteractionHandlerErrorKind> {
        let mut torrent_status = torrent_status.write().map_err(|err| {
            InteractionHandlerErrorKind::Unrecoverable(InteractionHandlerError::StoringBlock(
                format!("{}", err),
            ))
        })?;
        let piece_index = match torrent_status.look_for_a_missing_piece_index(&*self) {
            Some(piece_index) => {
                self.update_am_interested_field(torrent_file_data, true)?;
                piece_index
            }
            None => {
                self.update_am_interested_field(torrent_file_data, false)?;
                msg_sender::send_not_interested(&mut self.stream).map_err(|err| {
                    InteractionHandlerErrorKind::Recoverable(
                        InteractionHandlerError::SendingMessage(format!("{}", err)),
                    )
                })?;
                return Ok(());
            }
        };

        self.send_msg_according_to_peer_choking_field(
            piece_index,
            torrent_file_data,
            &mut torrent_status,
        )?;

        Ok(())
    }

    fn check_requested_block(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &TorrentStatus,
        piece_index: u32,
        beginning_byte_index: u32,
        amount_of_bytes: u32,
    ) -> Result<(), InteractionHandlerErrorKind> {
        let piece_index = usize::try_from(piece_index).map_err(|err| {
            InteractionHandlerErrorKind::Unrecoverable(
                InteractionHandlerError::SendingRequestedBlock(format!("{:?}", err)),
            )
        })?;

        if !torrent_status.is_a_valid_and_available_piece(piece_index) {
            return Err(InteractionHandlerErrorKind::Recoverable(
                InteractionHandlerError::SendingRequestedBlock(
                    "[InteractionHandlerError] The local peer does not have the requested block."
                        .to_string(),
                ),
            ));
        }
        if self.am_choking() {
            return Err(InteractionHandlerErrorKind::Recoverable(
                InteractionHandlerError::SendingRequestedBlock(
                    "[InteractionHandlerError] The external peer who send the request is choked."
                        .to_string(),
                ),
            ));
        }

        torrent_file_data
            .check_requested_block(piece_index, beginning_byte_index, amount_of_bytes)
            .map_err(|err| {
                InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::SendingRequestedBlock(format!("{:?}", err)),
                )
            })?;

        Ok(())
    }

    fn send_requested_block(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &Arc<RwLock<TorrentStatus>>,
        piece_index: u32,
        beginning_byte_index: u32,
        amount_of_bytes: u32,
    ) -> Result<(), InteractionHandlerErrorKind> {
        let temp_path_name = torrent_file_data.get_torrent_representative_name();
        let mut torrent_status = torrent_status.write().map_err(|err| {
            InteractionHandlerErrorKind::Unrecoverable(
                InteractionHandlerError::SendingRequestedBlock(format!("{}", err)),
            )
        })?;
        self.check_requested_block(
            torrent_file_data,
            &torrent_status,
            piece_index,
            beginning_byte_index,
            amount_of_bytes,
        )?;

        let block = block_handler::get_block(
            piece_index,
            beginning_byte_index,
            amount_of_bytes,
            &temp_path_name,
        )
        .map_err(|err| {
            InteractionHandlerErrorKind::Unrecoverable(
                InteractionHandlerError::SendingRequestedBlock(format!("{}", err)),
            )
        })?;

        msg_sender::send_piece(&mut self.stream, piece_index, beginning_byte_index, block)
            .map_err(|err| {
                InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::SendingRequestedBlock(format!("{}", err)),
                )
            })?;

        torrent_status.increment_uploaded_counter(amount_of_bytes.into());

        let upload_duration = self.clock.elapsed().map_err(|err| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::CalculatingTime(
                format!("{}", err),
            ))
        })?;
        ui_sender_handler::update_upload_data(
            &self.ui_sender,
            torrent_file_data,
            &self.external_peer_data,
            torrent_status.get_uploaded_bytes(),
            upload_duration,
        )
        .map_err(|err| {
            InteractionHandlerErrorKind::Recoverable(InteractionHandlerError::UiError(format!(
                "{}",
                err
            )))
        })?;

        Ok(())
    }

    fn send_msg_according_to_the_received_msg(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &Arc<RwLock<TorrentStatus>>,
        received_msg: &P2PMessage,
    ) -> Result<(), InteractionHandlerErrorKind> {
        match received_msg {
            P2PMessage::Interested => {
                msg_sender::send_unchoke(&mut self.stream).map_err(|err| {
                    InteractionHandlerErrorKind::Recoverable(
                        InteractionHandlerError::SendingMessage(format!("{}", err)),
                    )
                })?;
                self.external_peer_data.am_choking = false;
                Ok(())
            }
            P2PMessage::Request {
                piece_index,
                beginning_byte_index,
                amount_of_bytes,
            } => self.send_requested_block(
                torrent_file_data,
                torrent_status,
                *piece_index,
                *beginning_byte_index,
                *amount_of_bytes,
            ),
            P2PMessage::Cancel {
                piece_index: _,
                beginning_byte_index: _,
                amount_of_bytes: _,
            } => Ok(()),
            _ => Ok(()),
        }?;
        self.set_up_peer_roll_as_client()
    }

    fn react_according_to_the_peer_role(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &Arc<RwLock<TorrentStatus>>,
        received_msg: &P2PMessage,
    ) -> Result<(), InteractionHandlerErrorKind> {
        match self.role {
            PeerRole::Client => self.look_for_pieces(torrent_file_data, torrent_status),
            PeerRole::Server => self.send_msg_according_to_the_received_msg(
                torrent_file_data,
                torrent_status,
                received_msg,
            ),
        }
    }
}

#[cfg(test)]
mod test_client {
    use gtk::glib;

    use super::*;
    use std::{
        error::Error,
        fmt,
        net::{SocketAddr, TcpStream},
        str::FromStr,
        sync::mpsc,
        thread,
    };

    use crate::torrent::{
        data::{
            torrent_file_data::{TargetFilesData, TorrentFileData},
            torrent_status::{StateOfDownload, TorrentStatus},
            tracker_response_data::{PeerDataFromTrackerResponse, TrackerResponseData},
        },
        parsers::p2p::{
            constants::PSTR_STRING_HANDSHAKE,
            message::{P2PMessage, PieceStatus},
        },
        port_testing::listener_binder::*,
    };

    #[derive(PartialEq, Debug, Clone)]
    pub enum TestingError {
        ClientPeerFieldsInvalidAccess(String),
    }

    impl fmt::Display for TestingError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "\n    {:#?}\n", self)
        }
    }

    impl Error for TestingError {}

    pub const DEFAULT_ADDR: &str = "127.0.0.1:8080";
    pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
    pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";
    pub const DEFAULT_INFO_HASH: [u8; 20] = [0; 20];

    fn create_default_client_peer_with_unused_server_peer(
        num_of_test: u32,
    ) -> Result<
        (
            TrackerResponseData,
            Arc<RwLock<TorrentStatus>>,
            TorrentFileData,
            LocalPeerCommunicator,
            mpsc::Receiver<String>,
            glib::Receiver<MessageUI>,
        ),
        Box<dyn Error>,
    > {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;

        let handler = thread::spawn(move || listener.accept());

        let stream = TcpStream::connect(address.clone())?;
        let _joined = handler.join();

        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
            peer_address: SocketAddr::from_str(&address)?,
        };

        let tracker_response = TrackerResponseData {
            interval: 0,
            complete: 1,
            incomplete: 0,
            peers: vec![server_peer],
        };
        let torrent_status = TorrentStatus {
            uploaded: 0,
            downloaded: 0,
            left: 16,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::MissingPiece {
                was_requested: true,
            }],
        };
        let torrent_file = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: format!("message_related_block_storage{}.test", num_of_test),
                file_length: 16,
            },
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
            sha1_pieces: vec![],
            piece_length: 16,
            total_amount_of_pieces: 1,
            total_length: 16,
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            pieces_availability: vec![PieceStatus::ValidAndAvailablePiece],
            am_interested: false,
            am_choking: true,
            peer_choking: true,
            peer_interested: false,
        };

        let (logger_sender, _logger_receiver) = mpsc::channel();
        let (ui_sender, _ui_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let local_peer = LocalPeerCommunicator {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            stream,
            external_peer_data: server_peer_data,
            role: PeerRole::Client,
            logger_sender: logger_sender,
            ui_sender: ui_sender,
            clock: SystemTime::now(),
        };
        Ok((
            tracker_response,
            Arc::new(RwLock::new(torrent_status)),
            torrent_file,
            local_peer,
            _logger_receiver,
            _ui_receiver,
        ))
    }

    fn create_default_client_peer_with_a_server_peer_that_has_the_whole_file(
        num_of_test: u32,
    ) -> Result<
        (
            TrackerResponseData,
            Arc<RwLock<TorrentStatus>>,
            TorrentFileData,
            LocalPeerCommunicator,
            mpsc::Receiver<String>,
            glib::Receiver<MessageUI>,
        ),
        Box<dyn Error>,
    > {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;

        let handler = thread::spawn(move || listener.accept());

        let stream = TcpStream::connect(address.clone())?;
        let _joined = handler.join();

        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
            peer_address: SocketAddr::from_str(&address)?,
        };
        let tracker_response = TrackerResponseData {
            interval: 0,
            complete: 1,
            incomplete: 0,
            peers: vec![server_peer],
        };
        let torrent_status = TorrentStatus {
            uploaded: 0,
            downloaded: 0,
            left: 40000,
            event: StateOfDownload::Started,
            pieces_availability: vec![
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
            ],
        };
        let torrent_file = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: format!("message_related_block_storage{}.test", num_of_test),
                file_length: 40000,
                //1º pieza -> 34000 bytes
                //2º pieza ->  6000 bytes
            },
            sha1_pieces: vec![
                46, 101, 88, 42, 242, 153, 87, 30, 42, 117, 240, 135, 191, 37, 12, 42, 175, 156,
                136, 214, 95, 100, 198, 139, 237, 56, 161, 225, 113, 168, 52, 228, 26, 36, 103,
                150, 103, 76, 233, 34,
            ],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 34000,
            total_amount_of_pieces: 2,
            total_length: 40000,
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

        let (logger_sender, _logger_receiver) = mpsc::channel();
        let (ui_sender, _ui_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let local_peer = LocalPeerCommunicator {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            stream,
            external_peer_data: server_peer_data,
            role: PeerRole::Client,
            logger_sender: logger_sender,
            ui_sender: ui_sender,
            clock: SystemTime::now(),
        };
        Ok((
            tracker_response,
            Arc::new(RwLock::new(torrent_status)),
            torrent_file,
            local_peer,
            _logger_receiver,
            _ui_receiver,
        ))
    }

    fn create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces(
        num_of_test: u32,
    ) -> Result<
        (
            TrackerResponseData,
            Arc<RwLock<TorrentStatus>>,
            TorrentFileData,
            LocalPeerCommunicator,
            mpsc::Receiver<String>,
            glib::Receiver<MessageUI>,
        ),
        Box<dyn Error>,
    > {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;

        let handler = thread::spawn(move || listener.accept());

        let stream = TcpStream::connect(address.clone())?;
        let _joined = handler.join();

        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
            peer_address: SocketAddr::from_str(&address)?,
        };
        let tracker_response = TrackerResponseData {
            interval: 0,
            complete: 0,
            incomplete: 1,
            peers: vec![server_peer],
        };
        let torrent_status = TorrentStatus {
            uploaded: 0,
            downloaded: 0,
            left: 16,
            event: StateOfDownload::Started,
            pieces_availability: vec![
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
            ],
        };
        let torrent_file = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: format!("message_related_block_storage{}.test", num_of_test),
                file_length: 32,
            },
            sha1_pieces: vec![],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 16,
            total_amount_of_pieces: 2,
            total_length: 32,
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            pieces_availability: vec![
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
            ],
            am_interested: false,
            am_choking: true,
            peer_choking: true,
            peer_interested: false,
        };

        let (logger_sender, _logger_receiver) = mpsc::channel();
        let (ui_sender, _ui_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let local_peer = LocalPeerCommunicator {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            stream,
            external_peer_data: server_peer_data,
            role: PeerRole::Client,
            logger_sender: logger_sender,
            ui_sender: ui_sender,
            clock: SystemTime::now(),
        };
        Ok((
            tracker_response,
            Arc::new(RwLock::new(torrent_status)),
            torrent_file,
            local_peer,
            _logger_receiver,
            _ui_receiver,
        ))
    }

    fn create_default_client_with_a_piece_for_requests(
        address: String,
        num_of_test: u32,
    ) -> Result<
        (
            TrackerResponseData,
            Arc<RwLock<TorrentStatus>>,
            TorrentFileData,
            LocalPeerCommunicator,
            mpsc::Receiver<String>,
            glib::Receiver<MessageUI>,
        ),
        Box<dyn Error>,
    > {
        let stream = TcpStream::connect(address.clone())?;

        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
            peer_address: SocketAddr::from_str(&address)?,
        };
        let tracker_response = TrackerResponseData {
            interval: 0,
            complete: 0,
            incomplete: 0,
            peers: vec![server_peer],
        };
        let torrent_status = TorrentStatus {
            uploaded: 0,
            downloaded: 40000,
            left: 0,
            event: StateOfDownload::Started,
            pieces_availability: vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
            ],
        };
        let torrent_file = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: format!("send_requested_block_{}.test", num_of_test),
                file_length: 40000,
                //1º pieza -> 34000 bytes
                //2º pieza ->  6000 bytes
            },
            sha1_pieces: vec![
                46, 101, 88, 42, 242, 153, 87, 30, 42, 117, 240, 135, 191, 37, 12, 42, 175, 156,
                136, 214, 95, 100, 198, 139, 237, 56, 161, 225, 113, 168, 52, 228, 26, 36, 103,
                150, 103, 76, 233, 34,
            ],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            sha1_info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 34000,
            total_amount_of_pieces: 2,
            total_length: 40000,
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            pieces_availability: vec![
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
            ],
            am_interested: false,
            am_choking: true,
            peer_choking: true,
            peer_interested: false,
        };

        let (logger_sender, _logger_receiver) = mpsc::channel();
        let (ui_sender, _ui_receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        let local_peer = LocalPeerCommunicator {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            stream,
            external_peer_data: server_peer_data,
            role: PeerRole::Client,
            logger_sender: logger_sender,
            ui_sender: ui_sender,
            clock: SystemTime::now(),
        };
        Ok((
            tracker_response,
            Arc::new(RwLock::new(torrent_status)),
            torrent_file,
            local_peer,
            _logger_receiver,
            _ui_receiver,
        ))
    }

    mod test_generate_peer_data_from_handshake {
        use super::*;

        #[test]
        fn receive_a_message_that_is_not_a_handshake_error() -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let (
                tracker_response,
                _torrent_status,
                torrent_file_data,
                _,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_unused_server_peer(1)?;
            let message = P2PMessage::KeepAlive;

            assert!(generate_peer_data_from_handshake_torrent_peer(
                message,
                &torrent_file_data,
                &tracker_response,
                server_peer_index
            )
            .is_err());

            Ok(())
        }

        #[test]
        fn receive_a_handshake_with_an_incorrect_protocol_str_error() -> Result<(), Box<dyn Error>>
        {
            let server_peer_index = 0;
            let (
                tracker_response,
                _torrent_status,
                torrent_file_data,
                _,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_unused_server_peer(1)?;
            let message = P2PMessage::Handshake {
                protocol_str: "VitTorrent protocol".to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };

            assert!(generate_peer_data_from_handshake_torrent_peer(
                message,
                &torrent_file_data,
                &tracker_response,
                server_peer_index
            )
            .is_err());

            Ok(())
        }

        #[test]
        fn receive_a_handshake_with_an_incorrect_info_hash_error() -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let (
                tracker_response,
                _torrent_status,
                torrent_file_data,
                _,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_unused_server_peer(1)?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: [1; 20].to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };

            assert!(generate_peer_data_from_handshake_torrent_peer(
                message,
                &torrent_file_data,
                &tracker_response,
                server_peer_index
            )
            .is_err());

            Ok(())
        }

        #[test]
        fn receive_a_handshake_with_an_incorrect_peer_id_error() -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let (
                tracker_response,
                _torrent_status,
                torrent_file_data,
                _,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_unused_server_peer(1)?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: "-FA0001-000000000002".bytes().collect(),
            };

            assert!(generate_peer_data_from_handshake_torrent_peer(
                message,
                &torrent_file_data,
                &tracker_response,
                server_peer_index
            )
            .is_err());

            Ok(())
        }

        #[test]
        fn client_that_has_no_peer_ids_to_check_receive_a_valid_handshake_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;

            let (mut tracker_response, _, torrent_file_data, _, _log_receiver, _ui_receiver) =
                create_default_client_peer_with_unused_server_peer(1)?;

            //MODIFICO EL CLIENTE PARA QUE NO TENGA LOS PEER_ID DE LOS SERVER PEER
            tracker_response.peers = vec![PeerDataFromTrackerResponse {
                peer_id: None,
                peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
            }];

            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };
            let expected_peer_data = PeerDataForP2PCommunication {
                pieces_availability: vec![PieceStatus::MissingPiece {
                    was_requested: false,
                }],
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };

            let received_peer_data = generate_peer_data_from_handshake_torrent_peer(
                message,
                &torrent_file_data,
                &tracker_response,
                server_peer_index,
            )?;

            assert_eq!(expected_peer_data, received_peer_data);
            Ok(())
        }

        #[test]
        fn client_that_has_peer_ids_to_check_receive_a_valid_handshake_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let (
                tracker_response,
                _torrent_status,
                torrent_file_data,
                _,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_unused_server_peer(1)?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };
            let expected_peer_data = PeerDataForP2PCommunication {
                pieces_availability: vec![PieceStatus::MissingPiece {
                    was_requested: false,
                }],
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };

            let received_peer_data = generate_peer_data_from_handshake_torrent_peer(
                message,
                &torrent_file_data,
                &tracker_response,
                server_peer_index,
            )?;

            assert_eq!(expected_peer_data, received_peer_data);
            Ok(())
        }
    }

    mod test_update_peer_bitfield {
        use super::*;

        #[test]
        fn update_peer_bitfield_with_less_pieces_error() -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                _torrent_status,
                torrent_file_data,
                mut local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_unused_server_peer(1)?;
            let bitfield = vec![];

            assert!(local_peer
                .update_peer_bitfield(&torrent_file_data, &bitfield)
                .is_err());

            assert_eq!(local_peer.external_peer_data.pieces_availability.len(), 1);
            Ok(())
        }

        #[test]
        fn update_peer_bitfield_with_more_pieces_and_spare_bits_set_error(
        ) -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                _torrent_status,
                torrent_file_data,
                mut local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_unused_server_peer(1)?;
            let bitfield = vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::ValidAndAvailablePiece,
            ];

            assert!(local_peer
                .update_peer_bitfield(&torrent_file_data, &bitfield)
                .is_err());

            assert_eq!(local_peer.external_peer_data.pieces_availability.len(), 1);
            Ok(())
        }

        #[test]
        fn update_peer_bitfield_with_the_correct_amount_of_pieces_ok() -> Result<(), Box<dyn Error>>
        {
            let (
                _tracker_response,
                _torrent_status,
                torrent_file_data,
                mut local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_unused_server_peer(1)?;
            let bitfield = vec![PieceStatus::ValidAndAvailablePiece];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: vec![PieceStatus::MissingPiece {
                    was_requested: true,
                }],
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };
            local_peer.external_peer_data = peer_data;

            local_peer.update_peer_bitfield(&torrent_file_data, &bitfield)?;

            assert_eq!(
                vec![PieceStatus::ValidAndAvailablePiece],
                local_peer.external_peer_data.pieces_availability
            );
            Ok(())
        }

        #[test]
        fn update_peer_bitfield_with_more_pieces_and_spare_bits_not_set_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                _torrent_status,
                torrent_file_data,
                mut local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_unused_server_peer(1)?;
            let bitfield = vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
                PieceStatus::MissingPiece {
                    was_requested: true,
                },
            ];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: vec![PieceStatus::MissingPiece {
                    was_requested: true,
                }],
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };
            local_peer.external_peer_data = peer_data;

            local_peer.update_peer_bitfield(&torrent_file_data, &bitfield)?;

            assert_eq!(
                vec![PieceStatus::ValidAndAvailablePiece],
                local_peer.external_peer_data.pieces_availability
            );
            Ok(())
        }
    }

    mod test_update_server_peer_piece_status {

        use super::*;

        #[test]
        fn client_peer_update_piece_status_ok() -> Result<(), Box<dyn Error>> {
            let piece_index = 1;
            let (_, _, _torrent_file_data, mut local_peer, _log_receiver, _ui_receiver) =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces(1)?;

            local_peer.update_server_peer_piece_status(
                piece_index,
                PieceStatus::ValidAndAvailablePiece,
            )?;

            assert_eq!(
                local_peer
                    .external_peer_data
                    .pieces_availability
                    .get(piece_index),
                Some(&PieceStatus::ValidAndAvailablePiece)
            );
            Ok(())
        }

        #[test]
        fn client_peer_cannot_update_piece_status_with_invalid_index_error(
        ) -> Result<(), Box<dyn Error>> {
            let piece_index = 2;
            let (_, _, _, mut local_peer, _log_receiver, _ui_receiver) =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces(1)?;

            assert!(local_peer
                .update_server_peer_piece_status(piece_index, PieceStatus::ValidAndAvailablePiece,)
                .is_err());

            Ok(())
        }
    }

    mod test_store_block {
        use std::fs;

        use crate::torrent::client::peers_communication::handler_communication::BLOCK_BYTES;

        use super::*;

        #[test]
        fn the_received_block_is_smaller_than_expected_error() -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                torrent_status,
                torrent_file_data,
                local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_a_server_peer_that_has_the_whole_file(1)?;

            let piece_index = 0;
            let beginning_byte_index = 0;
            let block = vec![];
            let path = "test_client/store_block_1".to_string();

            assert_eq!(
                Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::StoringBlock(
                        "[InteractionHandlerError] Block length is not as expected".to_string()
                    )
                )),
                local_peer.store_block(
                    &torrent_file_data,
                    &torrent_status,
                    &path,
                    piece_index,
                    beginning_byte_index,
                    block
                )
            );

            Ok(())
        }

        #[test]
        fn the_received_block_is_bigger_than_expected_error() -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                torrent_status,
                torrent_file_data,
                local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_a_server_peer_that_has_the_whole_file(2)?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block = [0; BLOCK_BYTES as usize + 1].to_vec();
            let path = "test_client/store_block_2".to_string();

            assert_eq!(
                Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::StoringBlock(
                        "[InteractionHandlerError] Block length is not as expected".to_string()
                    )
                )),
                local_peer.store_block(
                    &torrent_file_data,
                    &torrent_status,
                    &path,
                    piece_index,
                    beginning_byte_index,
                    block
                )
            );

            Ok(())
        }

        #[test]
        fn the_received_piece_index_is_invalid_error() -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                torrent_status,
                torrent_file_data,
                local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_a_server_peer_that_has_the_whole_file(3)?;
            let piece_index = 2;
            let beginning_byte_index = 0;
            let block = [0; BLOCK_BYTES as usize].to_vec();
            let path = "test_client/store_block_3".to_string();

            assert_eq!(
                Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::StoringBlock(
                        "[InteractionHandlerError] The received piece index is invalid."
                            .to_string(),
                    )
                )),
                local_peer.store_block(
                    &torrent_file_data,
                    &torrent_status,
                    &path,
                    piece_index,
                    beginning_byte_index,
                    block
                )
            );

            Ok(())
        }

        #[test]
        fn the_client_peer_receives_one_block_ok() -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                torrent_status,
                torrent_file_data,
                local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_a_server_peer_that_has_the_whole_file(4)?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block = [0; 16384].to_vec();
            let path = "test_client/store_block_4".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            let storing_result = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block,
            );

            assert_eq!(storing_result, Ok(()));

            if let Some(PieceStatus::PartiallyDownloaded {
                downloaded_bytes, ..
            }) = torrent_status
                .read()
                .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                .pieces_availability
                .get(piece_index as usize)
            {
                assert_eq!(BLOCK_BYTES, *downloaded_bytes);
                fs::remove_dir_all(format!("temp/{}", path))?;
                return Ok(());
            }

            fs::remove_dir_all(format!("temp/{}", path))?;
            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn the_client_peer_receives_an_entire_piece_ok() -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                torrent_status,
                torrent_file_data,
                local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_a_server_peer_that_has_the_whole_file(5)?;
            let piece_index = 1;
            let beginning_byte_index = 0;
            let block = [0; 6000].to_vec();
            let path = "test_client/store_block_5".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            let storing_result = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block,
            );

            assert_eq!(storing_result, Ok(()));

            if let Some(piece_status) = torrent_status
                .read()
                .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                .pieces_availability
                .get(piece_index as usize)
            {
                assert_eq!(PieceStatus::ValidAndAvailablePiece, *piece_status);
                fs::remove_dir_all(format!("temp/{}", path))?;
                return Ok(());
            }

            fs::remove_dir_all(format!("temp/{}", path))?;
            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn the_client_peer_receives_a_piece_that_already_own_ok() -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                torrent_status,
                torrent_file_data,
                local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_a_server_peer_that_has_the_whole_file(6)?;
            let piece_index = 1;
            let mut beginning_byte_index = 0;
            let block = [0; 6000].to_vec();
            let path = "test_client/store_block_6".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            let storing_result = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block.clone(),
            );

            assert_eq!(storing_result, Ok(()));

            beginning_byte_index = 6000;
            assert_eq!(
                Ok(()),
                local_peer.store_block(
                    &torrent_file_data,
                    &torrent_status,
                    &path,
                    piece_index,
                    beginning_byte_index,
                    block
                )
            );
            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn the_client_peer_receives_two_blocks_ok() -> Result<(), Box<dyn Error>> {
            let (_, torrent_status, torrent_file_data, local_peer, _log_receiver, _ui_receiver) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file(7)?;
            let piece_index = 0;
            let mut beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let path = "test_client/store_block_7".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            let storing_result_1 = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_1,
            );
            assert_eq!(storing_result_1, Ok(()));

            beginning_byte_index = 16384;
            torrent_status
                .write()
                .map_err(|err| TestingError::ClientPeerFieldsInvalidAccess(format!("{:?}", err)))?
                .set_piece_as_requested(piece_index)?;
            let storing_result_2 = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_2,
            );

            assert_eq!(storing_result_2, Ok(()));

            if let Some(PieceStatus::PartiallyDownloaded {
                downloaded_bytes, ..
            }) = torrent_status
                .read()
                .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                .pieces_availability
                .get(piece_index as usize)
            {
                assert_eq!(BLOCK_BYTES * 2, *downloaded_bytes);
                fs::remove_dir_all(format!("temp/{}", path))?;
                return Ok(());
            }

            fs::remove_dir_all(format!("temp/{}", path))?;
            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn the_client_peer_receives_three_blocks_and_completes_a_piece_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                torrent_status,
                torrent_file_data,
                local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_a_server_peer_that_has_the_whole_file(8)?;
            let piece_index = 0;
            let mut beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let block_3 = [0; 34000 - (2 * 16384)].to_vec();
            let path = "test_client/store_block_8".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            let storing_result_1 = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_1,
            );
            assert_eq!(storing_result_1, Ok(()));

            beginning_byte_index = 16384;
            torrent_status
                .write()
                .map_err(|err| TestingError::ClientPeerFieldsInvalidAccess(format!("{:?}", err)))?
                .set_piece_as_requested(piece_index)?;
            let storing_result_2 = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_2,
            );
            assert_eq!(storing_result_2, Ok(()));

            beginning_byte_index = 16384 * 2;
            torrent_status
                .write()
                .map_err(|err| TestingError::ClientPeerFieldsInvalidAccess(format!("{:?}", err)))?
                .set_piece_as_requested(piece_index)?;
            let storing_result_3 = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_3,
            );
            assert_eq!(storing_result_3, Ok(()));

            if let Some(piece_status) = torrent_status
                .read()
                .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                .pieces_availability
                .get(piece_index as usize)
            {
                assert_eq!(PieceStatus::ValidAndAvailablePiece, *piece_status);
                fs::remove_dir_all(format!("temp/{}", path))?;
                return Ok(());
            }

            fs::remove_dir_all(format!("temp/{}", path))?;
            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn the_client_peer_receives_two_blocks_with_an_incorrect_beginning_byte_index_error(
        ) -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                torrent_status,
                torrent_file_data,
                local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_a_server_peer_that_has_the_whole_file(9)?;
            let piece_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let beginning_byte_index1 = 0;
            let beginning_byte_index2 = 20000;
            let path = "test_client/store_block_9".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            let storing_result = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index1,
                block_1.clone(),
            );
            assert_eq!(storing_result, Ok(()));

            torrent_status
                .write()
                .map_err(|err| TestingError::ClientPeerFieldsInvalidAccess(format!("{:?}", err)))?
                .set_piece_as_requested(piece_index)?;
            assert_eq!(
                Ok(()),
                local_peer.store_block(
                    &torrent_file_data,
                    &torrent_status,
                    &path,
                    piece_index,
                    beginning_byte_index1,
                    block_1
                )
            );

            torrent_status
                .write()
                .map_err(|err| TestingError::ClientPeerFieldsInvalidAccess(format!("{:?}", err)))?
                .set_piece_as_requested(piece_index)?;
            assert_eq!(
                Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::StoringBlock(
                        "[InteractionHandlerError] The beginning byte index is incorrect."
                            .to_string(),
                    )
                )),
                local_peer.store_block(
                    &torrent_file_data,
                    &torrent_status,
                    &path,
                    piece_index,
                    beginning_byte_index2,
                    block_2
                )
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn the_client_peer_receives_three_blocks_and_updates_downloaded_data_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (
                _tracker_response,
                torrent_status,
                torrent_file_data,
                local_peer,
                _log_receiver,
                _ui_receiver,
            ) = create_default_client_peer_with_a_server_peer_that_has_the_whole_file(10)?;
            let piece_index = 0;
            let mut beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let block_3 = [0; 34000 - (2 * 16384)].to_vec();
            let path = "test_client/store_block_10".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            assert_eq!(
                0,
                torrent_status
                    .read()
                    .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                    .downloaded
            );
            assert_eq!(
                40000,
                torrent_status
                    .read()
                    .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                    .left
            );

            let storing_result = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_1,
            );
            assert_eq!(storing_result, Ok(()));

            assert_eq!(
                16384,
                torrent_status
                    .read()
                    .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                    .downloaded
            );
            assert_eq!(
                40000 - 16384,
                torrent_status
                    .read()
                    .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                    .left
            );

            beginning_byte_index = 16384;
            torrent_status
                .write()
                .map_err(|err| TestingError::ClientPeerFieldsInvalidAccess(format!("{:?}", err)))?
                .set_piece_as_requested(piece_index)?;
            let storing_result = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_2,
            );
            assert_eq!(storing_result, Ok(()));

            assert_eq!(
                16384 * 2,
                torrent_status
                    .read()
                    .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                    .downloaded
            );
            assert_eq!(
                40000 - 16384 * 2,
                torrent_status
                    .read()
                    .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                    .left
            );

            beginning_byte_index = 16384 * 2;
            torrent_status
                .write()
                .map_err(|err| TestingError::ClientPeerFieldsInvalidAccess(format!("{:?}", err)))?
                .set_piece_as_requested(piece_index)?;
            let storing_result = local_peer.store_block(
                &torrent_file_data,
                &torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_3,
            );
            assert_eq!(storing_result, Ok(()));

            assert_eq!(
                34000,
                torrent_status
                    .read()
                    .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                    .downloaded
            );
            assert_eq!(
                40000 - 34000,
                torrent_status
                    .read()
                    .map_err(|err| InteractionHandlerError::LockingTorrentStatus(err.to_string()))?
                    .left
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }
    }

    mod test_send_requested_block {
        use std::fs;

        use crate::torrent::client::peers_communication::handler_communication::BLOCK_BYTES;

        use super::*;

        #[test]
        fn local_peer_does_not_have_the_requested_block_error() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let (_, torrent_status, torrent_file_data, mut local_peer, _log_receiver, _ui_receiver) =
                create_default_client_with_a_piece_for_requests(address, 1)?;
            let (_, _) = listener.accept()?;

            let block_0 = [10; BLOCK_BYTES as usize].to_vec();
            let block_1 = [10; BLOCK_BYTES as usize].to_vec();
            let block_2 = [10; (34000 - 2 * BLOCK_BYTES) as usize].to_vec();

            let piece_index = 1;
            let beginning_byte_index = 32;
            let amount_of_bytes = 16;

            let path = "test_client/send_requested_block_1".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            block_handler::store_block(&block_0, piece_index, &path)?;
            block_handler::store_block(&block_1, piece_index, &path)?;
            block_handler::store_block(&block_2, piece_index, &path)?;

            assert_eq!(
                local_peer.send_requested_block(
                    &torrent_file_data,
                    &torrent_status,
                    piece_index.try_into()?,
                    beginning_byte_index,
                    amount_of_bytes,
                ),
                Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::SendingRequestedBlock(
                        "[InteractionHandlerError] The local peer does not have the requested block."
                            .to_string(),
                    ),
                ))
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn peer_who_request_block_is_chocked_error() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let (_, torrent_status, torrent_file_data, mut local_peer, _log_receiver, _ui_receiver) =
                create_default_client_with_a_piece_for_requests(address, 2)?;
            let (_, _) = listener.accept()?;

            let block_0 = [10; BLOCK_BYTES as usize].to_vec();
            let block_1 = [10; BLOCK_BYTES as usize].to_vec();
            let block_2 = [10; (34000 - 2 * BLOCK_BYTES) as usize].to_vec();

            let piece_index = 0;
            let beginning_byte_index = 0;
            let amount_of_bytes = BLOCK_BYTES;

            let path = "test_client/send_requested_block_2".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            block_handler::store_block(&block_0, piece_index, &path)?;
            block_handler::store_block(&block_1, piece_index, &path)?;
            block_handler::store_block(&block_2, piece_index, &path)?;

            assert_eq!(
                local_peer.send_requested_block(
                    &torrent_file_data,
                    &torrent_status,
                    piece_index.try_into()?,
                    beginning_byte_index,
                    amount_of_bytes,
                ),
                Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::SendingRequestedBlock(
                        "[InteractionHandlerError] The external peer who send the request is choked."
                            .to_string(),
                    ),
                ))
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn beginning_byte_index_is_bigger_than_the_piece_length_error() -> Result<(), Box<dyn Error>>
        {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let (_, torrent_status, torrent_file_data, mut local_peer, _log_receiver, _ui_receiver) =
                create_default_client_with_a_piece_for_requests(address, 3)?;
            let (_, _) = listener.accept()?;
            local_peer.external_peer_data.am_choking = false;

            let block_0 = [10; BLOCK_BYTES as usize].to_vec();
            let block_1 = [10; BLOCK_BYTES as usize].to_vec();
            let block_2 = [10; (34000 - 2 * BLOCK_BYTES) as usize].to_vec();

            let piece_index = 0;
            let beginning_byte_index = 34000;
            let amount_of_bytes = 1;

            let path = "test_client/send_requested_block_3".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            block_handler::store_block(&block_0, piece_index, &path)?;
            block_handler::store_block(&block_1, piece_index, &path)?;
            block_handler::store_block(&block_2, piece_index, &path)?;

            assert_eq!(
                local_peer.send_requested_block(
                    &torrent_file_data,
                    &torrent_status,
                    piece_index.try_into()?,
                    beginning_byte_index,
                    amount_of_bytes,
                ),
                Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::SendingRequestedBlock(
                        "CheckingRequestBlock(\"[TorrentFileDataError] The requested amount of bytes does not match with piece lenght.\")"
                            .to_string(),
                    ),
                ))
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn invalid_requested_amount_of_bytes_error() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let (_, torrent_status, torrent_file_data, mut local_peer, _log_receiver, _ui_receiver) =
                create_default_client_with_a_piece_for_requests(address, 4)?;
            let (_, _) = listener.accept()?;
            local_peer.external_peer_data.am_choking = false;

            let block_0 = [10; BLOCK_BYTES as usize].to_vec();
            let block_1 = [10; BLOCK_BYTES as usize].to_vec();
            let block_2 = [10; (34000 - 2 * BLOCK_BYTES) as usize].to_vec();

            let piece_index = 0;
            let beginning_byte_index = BLOCK_BYTES * 2;
            let amount_of_bytes = BLOCK_BYTES;

            let path = "test_client/send_requested_block_4".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            block_handler::store_block(&block_0, piece_index, &path)?;
            block_handler::store_block(&block_1, piece_index, &path)?;
            block_handler::store_block(&block_2, piece_index, &path)?;

            assert_eq!(
                local_peer.send_requested_block(
                    &torrent_file_data,
                    &torrent_status,
                    piece_index.try_into()?,
                    beginning_byte_index,
                    amount_of_bytes,
                ),
                Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::SendingRequestedBlock(
                        "CheckingRequestBlock(\"[TorrentFileDataError] The requested amount of bytes does not match with piece lenght.\")"
                            .to_string(),
                    ),
                ))
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn requested_amount_of_bytes_is_bigger_than_16kbytes_error() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let (_, torrent_status, torrent_file_data, mut local_peer, _log_receiver, _ui_receiver) =
                create_default_client_with_a_piece_for_requests(address, 5)?;
            let (_, _) = listener.accept()?;
            local_peer.external_peer_data.am_choking = false;

            let block_0 = [10; BLOCK_BYTES as usize].to_vec();
            let block_1 = [10; BLOCK_BYTES as usize].to_vec();
            let block_2 = [10; (34000 - 2 * BLOCK_BYTES) as usize].to_vec();

            let piece_index = 0;
            let beginning_byte_index = 0;
            let amount_of_bytes = BLOCK_BYTES + 1;

            let path = "test_client/send_requested_block_5".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            block_handler::store_block(&block_0, piece_index, &path)?;
            block_handler::store_block(&block_1, piece_index, &path)?;
            block_handler::store_block(&block_2, piece_index, &path)?;

            assert_eq!(
                local_peer.send_requested_block(
                    &torrent_file_data,
                    &torrent_status,
                    piece_index.try_into()?,
                    beginning_byte_index,
                    amount_of_bytes,
                ),
                Err(InteractionHandlerErrorKind::Recoverable(
                    InteractionHandlerError::SendingRequestedBlock(
                        "CheckingRequestBlock(\"[TorrentFileDataError] The requested amount of bytes is bigger than 2^14 bytes.\")"
                            .to_string(),
                    ),
                ))
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn local_peer_sends_a_request_ok() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let (_, torrent_status, torrent_file_data, mut local_peer, _log_receiver, _ui_receiver) =
                create_default_client_with_a_piece_for_requests(address, 6)?;
            let (mut external_stream, _) = listener.accept()?;
            local_peer.external_peer_data.am_choking = false;

            let block_0 = [10; BLOCK_BYTES as usize].to_vec();
            let block_1 = [10; BLOCK_BYTES as usize].to_vec();
            let block_2 = [10; (34000 - 2 * BLOCK_BYTES) as usize].to_vec();

            let piece_index = 0;
            let beginning_byte_index = BLOCK_BYTES;
            let amount_of_bytes = BLOCK_BYTES;

            let path = torrent_file_data.get_torrent_representative_name();
            fs::create_dir(format!("temp/{}", path))?;

            block_handler::store_block(&block_0, piece_index, &path)?;
            block_handler::store_block(&block_1, piece_index, &path)?;
            block_handler::store_block(&block_2, piece_index, &path)?;

            local_peer.send_requested_block(
                &torrent_file_data,
                &torrent_status,
                piece_index.try_into()?,
                beginning_byte_index,
                amount_of_bytes,
            )?;

            let received_msg = msg_receiver::receive_message(&mut external_stream)?;
            let expected_msg = P2PMessage::Piece {
                piece_index: piece_index.try_into()?,
                beginning_byte_index,
                block: block_1,
            };

            assert_eq!(expected_msg, received_msg);

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }
    }
}
