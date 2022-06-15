//! # Modulo de manejo general de la estructura principal: Client
//! Este modulo contiene las funciones encargadas del comportamiento general
//! de nuestro cliente como peer de tipo leecher.

use crate::torrent::{
    client::{
        block_handler,
        medatada_analyzer::{read_torrent_file_to_dic, MetadataError},
        peers_comunication::{
            msg_logic_control::MsgLogicControlError,
            msg_receiver::{self, MsgReceiverError},
            msg_sender::{self, MsgSenderError},
        },
        tracker_comunication::http_handler::ErrorMsgHttp,
        tracker_comunication::http_handler::HttpHandler,
    },
    data::{
        peer_data_for_communication::PeerDataForP2PCommunication,
        torrent_file_data::{TorrentFileData, TorrentFileDataError},
        torrent_status::TorrentStatus,
        tracker_response_data::{ResponseError, TrackerResponseData},
    },
    parsers::p2p::{
        constants::PSTR_STRING_HANDSHAKE,
        message::{P2PMessage, PieceStatus},
    },
};
extern crate rand;
use log::{debug, error, info, trace};
use rand::{distributions::Alphanumeric, Rng};
use std::{error::Error, fmt, net::TcpStream, time::Duration};

use super::client::peers_comunication::handler::HandlerInteractionStatus;

const SIZE_PEER_ID: usize = 12;
const INIT_PEER_ID: &str = "-FA0000-";

pub const SECS_READ_TIMEOUT: u64 = 120;
pub const NANOS_READ_TIMEOUT: u32 = 0;

type ResultClient<T> = Result<T, ClientError>;

#[derive(PartialEq, Debug, Clone)]
pub enum PeerRole {
    Client,
    Server,
}

#[derive(Debug)]
/// Struct que tiene por comportamiento todo el manejo general de actualizacion importante de datos, almacenamiento de los mismos y ejecución de metodos importantes para la comunicación con peers durante la ejecución del programa a modo de leecher.
pub struct LocalPeer {
    pub peer_id: Vec<u8>,
    pub stream: TcpStream,
    pub external_peer_data: PeerDataForP2PCommunication,
    pub role: PeerRole,
}

#[derive(PartialEq, Debug)]
// Representa posibles errores durante la ejecucion de alguna de sus funcionalidades
pub enum ClientError {
    File(MetadataError),
    HttpCreation(ErrorMsgHttp),
    ConectionError(ErrorMsgHttp),
    TorrentCreation(TorrentFileDataError),
    Response(ResponseError),
    PeerCommunication(MsgLogicControlError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error del torrent.\n Backtrace: {:?}\n", self)
    }
}

impl Error for ClientError {}

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
    tracker_response_data: &TrackerResponseData,
    tracker_response_peer_index: usize,
) -> Result<TcpStream, MsgLogicControlError> {
    if let Some(peer_address) = tracker_response_data.get_peer_address(tracker_response_peer_index)
    {
        let stream = TcpStream::connect(peer_address)
            .map_err(|error| MsgLogicControlError::ConectingWithPeer(format!("{:?}", error)))?;

        stream
            .set_read_timeout(Some(Duration::new(SECS_READ_TIMEOUT, NANOS_READ_TIMEOUT)))
            .map_err(|err| MsgLogicControlError::ConectingWithPeer(format!("{:?}", err)))?;
        return Ok(stream);
    }

    Err(MsgLogicControlError::ConectingWithPeer(String::from(
        "[MsgLogicControlError] Client peer doesn`t have a tracker response.",
    )))
}
//HANDSHAKE
fn check_handshake(
    torrent_file_data: &TorrentFileData,
    tracker_response_data: &TrackerResponseData,
    server_protocol_str: String,
    server_info_hash: &[u8],
    server_peer_id: &[u8],
    tracker_response_peer_index: usize,
) -> Result<(), MsgLogicControlError> {
    if (server_protocol_str == PSTR_STRING_HANDSHAKE)
        && torrent_file_data.has_expected_info_hash(server_info_hash)
        && tracker_response_data.has_expected_peer_id(tracker_response_peer_index, server_peer_id)
    {
        Ok(())
    } else {
        Err(MsgLogicControlError::CheckingAndSavingHandshake(
            "[MsgLogicControlError] The received handshake hasn`t got the expected fields."
                .to_string(),
        ))
    }
}

/// Funcion que realiza la verificacion de un mensaje recibido de tipo
/// Handshake y almacena su info importante
///
pub fn generate_peer_data_from_handshake(
    message: P2PMessage,
    torrent_file_data: &TorrentFileData,
    tracker_response_data: &TrackerResponseData,
    tracker_response_peer_index: usize,
) -> Result<PeerDataForP2PCommunication, MsgLogicControlError> {
    if let P2PMessage::Handshake {
        protocol_str: server_protocol_str,
        info_hash: server_info_hash,
        peer_id: server_peer_id,
    } = message
    {
        check_handshake(
            torrent_file_data,
            tracker_response_data,
            server_protocol_str,
            &server_info_hash,
            &server_peer_id,
            tracker_response_peer_index,
        )?;
        Ok(PeerDataForP2PCommunication::new(
            torrent_file_data,
            server_peer_id,
        ))
    } else {
        Err(MsgLogicControlError::CheckingAndSavingHandshake(
            "[MsgLogicControlError] The received messagge is not a handshake.".to_string(),
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

// --------------------------------------------------

/// Funcion que lee toda la metadata y almacena su información importante
///
pub fn create_torrent(torrent_path: &str) -> ResultClient<TorrentFileData> {
    trace!("Leyendo el archivo para poder crear el torrent");
    let torrent_dic = match read_torrent_file_to_dic(torrent_path) {
        Ok(dictionary) => dictionary,
        Err(error) => {
            error!("Error del cliente al leer archivo y pasarlo a HashMap");
            return Err(ClientError::File(error));
        }
    };
    trace!("Arhivo leido y pasado a HashMap exitosamente");
    trace!("Creando TorrentFileData");
    match TorrentFileData::new(torrent_dic) {
        Ok(torrent) => Ok(torrent),
        Err(error) => {
            error!("Error del cliente al crear la estructura del torrent");
            Err(ClientError::TorrentCreation(error))
        }
    }
}

/// Funcion que realiza toda la comunicación con el tracker, interpreta su
/// respuesta y devuelve la info importante de la misma
///
pub fn communicate_with_tracker(torrent: &TorrentFileData) -> ResultClient<TrackerResponseData> {
    let str_peer_id = String::from_utf8_lossy(&generate_peer_id()).to_string();
    trace!("Creando httpHandler dentro del Client");
    let http_handler = match HttpHandler::new(torrent, str_peer_id) {
        Ok(http) => http,
        Err(error) => {
            error!("Error del cliente al crear HttpHandler");
            return Err(ClientError::HttpCreation(error));
        }
    };
    trace!("HttpHandler creado exitosamente");
    trace!("Comunicacion con el Tracker mediante httpHandler");
    let response_tracker = match http_handler.tracker_get_response() {
        Ok(response) => response,
        Err(error) => {
            return {
                error!("Error del cliente al conectarse con el Tracker");
                Err(ClientError::ConectionError(error))
            }
        }
    };
    trace!("Creando el TrackerResponseData en base a la respues del tracker");
    match TrackerResponseData::new(response_tracker) {
        Ok(response_struct) => Ok(response_struct),
        Err(error) => {
            error!("Error del cliente al recibir respuesta del Tracker");
            Err(ClientError::Response(error))
        }
    }
}

impl LocalPeer {
    /// Funcion que interpreta toda la info del .torrent, se comunica con el
    /// tracker correspondiente y almacena todos los datos importantes para
    /// su uso posterior en comunicacion con peers, devolviendo así
    /// una instancia de la estructura lista para ello.
    ///
    pub fn new(
        external_peer_data: PeerDataForP2PCommunication,
        stream: TcpStream,
        role: PeerRole,
    ) -> ResultClient<Self> {
        trace!("Genero peer_id");
        let peer_id = generate_peer_id();

        // Los comento porque despues hay que meterlos en la funcion principal de la app (el run())
        // let torrent_file = create_torrent(path_file)?;
        // trace!("TorrentFileData creado y almacenado dentro del Client");
        // let info_hash = torrent_file.get_info_hash();
        // let torrent_size = torrent_file.get_total_size() as u64;
        // let torrent_status = TorrentStatus::new(torrent_size, torrent_file.total_amount_of_pieces);
        Ok(LocalPeer {
            peer_id,
            stream,
            external_peer_data,
            role,
        })
    }

    pub fn start_communication(
        torrent_file_data: &TorrentFileData,
        tracker_response_data: &TrackerResponseData,
        tracker_response_peer_index: usize,
    ) -> Result<Self, MsgLogicControlError> {
        //GENERO PEER ID
        let peer_id = generate_peer_id();
        trace!("Genero peer_id");

        //CONEXION CON UN PEER
        let mut local_peer_stream =
            open_connection_with_peer(tracker_response_data, tracker_response_peer_index)?;
        info!("El cliente se conecta con un peer exitosamente.");

        //ENVIO HANDSHAKE
        msg_sender::send_handshake(&mut local_peer_stream, &peer_id, torrent_file_data).map_err(
            |error| {
                if let MsgSenderError::WriteToTcpStream(_) = error {
                    MsgLogicControlError::ConectingWithPeer(format!("{:?}", error))
                } else {
                    MsgLogicControlError::SendingHandshake(format!("{:?}", error))
                }
            },
        )?;
        info!("Mensaje enviado: Handshake.");

        //RECIBO HANDSHAKE
        let received_handshake =
            msg_receiver::receive_handshake(&mut local_peer_stream).map_err(|error| {
                if let MsgReceiverError::ReadingFromTcpStream(_) = error {
                    MsgLogicControlError::ConectingWithPeer(format!("{:?}", error))
                } else {
                    MsgLogicControlError::ReceivingHanshake(format!("{:?}", error))
                }
            })?;
        info!("Mensaje recibido: Handshake.");

        let external_peer_data = generate_peer_data_from_handshake(
            received_handshake,
            torrent_file_data,
            tracker_response_data,
            tracker_response_peer_index,
        )?;

        Ok(LocalPeer {
            peer_id,
            stream: local_peer_stream,
            external_peer_data,
            role: PeerRole::Client,
        })
    }

    /// Funcion que realiza toda la comunicación con el tracker, interpreta su
    /// respuesta y almacena la info importante de la misma
    ///
    // pub fn communicate_with_tracker(&mut self) -> ResultClient<()> {
    //     match communicate_with_tracker(self.torrent_file.clone()) {
    //         Ok(response) => self.tracker_response = Some(response),
    //         Err(error) => return Err(error),
    //     };
    //     trace!("TrackerResponseData creado y almacenado dentro del Client");
    //     Ok(())
    // }
    // Miguel: creo que no tiene mucho sentido tener esto como metodo del LocalPeer

    //BITFIELD
    /// Funcion que actualiza la representación de bitfield de un peer dado
    /// por su indice
    ///
    pub fn update_peer_bitfield(
        &mut self,
        torrent_file_data: &TorrentFileData,
        mut bitfield: Vec<PieceStatus>,
    ) -> Result<(), MsgLogicControlError> {
        torrent_file_data
            .check_bitfield(&bitfield)
            .map_err(|err| MsgLogicControlError::UpdatingBitfield(format!("{:?}", err)))?;
        bitfield.truncate(torrent_file_data.get_total_amount_pieces());
        self.external_peer_data.update_pieces_availability(bitfield);
        Ok(())
    }

    // HAVE
    /// Funcion que actualiza la representación de bitfield de un peer dado
    /// por su indice (A diferencia de [update_peer_bitfield()], esta funcion
    /// actualiza solo el estado de UNA pieza, esto es causado por
    /// la recepcion de un mensaje P2P de tipo Have)
    ///
    pub fn update_server_peer_piece_status(
        &mut self,
        piece_index: usize,
        new_status: PieceStatus,
    ) -> Result<(), MsgLogicControlError> {
        self.external_peer_data
            .update_piece_status(piece_index, new_status)
            .map_err(|err| MsgLogicControlError::UpdatingPieceStatus(format!("{:?}", err)))?;
        Ok(())
    }

    //PIECE
    fn check_piece_index_and_beginning_byte_index(
        &self,
        torrent_status: &TorrentStatus,
        piece_index: usize,
        beginning_byte_index: u32,
    ) -> Result<(), MsgLogicControlError> {
        match torrent_status.get_piece_status(piece_index) {
            Some(piece_status) => match *piece_status {
                PieceStatus::ValidAndAvailablePiece => {
                    return Err(MsgLogicControlError::StoringBlock(
                        "[MsgLogicControlError] The client peer has already completed that piece."
                            .to_string(),
                    ))
                }
                PieceStatus::PartiallyDownloaded { downloaded_bytes } => {
                    if downloaded_bytes != beginning_byte_index {
                        return Err(MsgLogicControlError::StoringBlock(
                            "[MsgLogicControlError] The beginning byte index is incorrect."
                                .to_string(),
                        ));
                    }
                }
                _ => (),
            },
            None => {
                return Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] The received piece index is invalid.".to_string(),
                ))
            }
        };
        Ok(())
    }

    fn check_block_lenght(
        &self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &TorrentStatus,
        piece_index: usize,
        beginning_byte_index: u32,
        block: &[u8],
    ) -> Result<(), MsgLogicControlError> {
        let expected_amount_of_bytes = torrent_status
            .calculate_amount_of_bytes_of_block(
                torrent_file_data,
                piece_index,
                beginning_byte_index,
            )
            .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?
            .try_into()
            .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?;
        if block.len() != expected_amount_of_bytes {
            return Err(MsgLogicControlError::StoringBlock(
                "[MsgLogicControlError] Block length is not as expected".to_string(),
            ));
        }
        Ok(())
    }

    fn check_store_block(
        &self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &TorrentStatus,
        piece_index: usize,
        beginning_byte_index: u32,
        block: &[u8],
    ) -> Result<(), MsgLogicControlError> {
        self.check_piece_index_and_beginning_byte_index(
            torrent_status,
            piece_index,
            beginning_byte_index,
        )?;
        self.check_block_lenght(
            torrent_file_data,
            torrent_status,
            piece_index,
            beginning_byte_index,
            block,
        )?;
        Ok(())
    }

    fn check_piece(
        &self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &mut TorrentStatus,
        path: &str,
        piece_index: usize,
    ) -> Result<(), MsgLogicControlError> {
        if torrent_status.is_a_valid_and_available_piece(piece_index) {
            info!("Se completó la pieza {}.", piece_index);
            info!("Verifico el hash SHA1 de la pieza descargada.");
            block_handler::check_sha1_piece(torrent_file_data, piece_index, path)
                .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?;
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
    pub fn store_block(
        &self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &mut TorrentStatus,
        path: &str,
        piece_index: usize,
        beginning_byte_index: u32,
        block: Vec<u8>,
    ) -> Result<(), MsgLogicControlError> {
        self.check_store_block(
            torrent_file_data,
            &*torrent_status,
            piece_index,
            beginning_byte_index,
            &block,
        )?;
        block_handler::store_block(&block, piece_index, path)
            .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?;

        torrent_status
            .update_piece_status(
                torrent_file_data,
                piece_index,
                beginning_byte_index,
                u32::try_from(block.len())
                    .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?,
            )
            .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?;

        self.check_piece(torrent_file_data, torrent_status, path, piece_index)?;
        Ok(())
    }

    // UPDATING FIELDS

    /// Funcion que actualiza si mi cliente tiene chokeado a un peer especifico
    ///
    pub fn update_am_choking_field(&mut self, new_value: bool) -> Result<(), MsgLogicControlError> {
        self.external_peer_data.am_choking = new_value;
        Ok(())
    }

    /// Funcion que actualiza si el cliente está interesado en una pieza
    /// de un peer dado por su indice.
    ///
    pub fn update_am_interested_field(
        &mut self,
        new_value: bool,
    ) -> Result<(), MsgLogicControlError> {
        self.external_peer_data.am_interested = new_value;
        Ok(())
    }

    /// Funcion que actualiza si un peer me tiene chokeado a mi cliente
    ///
    pub fn update_peer_choking_field(
        //esta funcion ya no sirve para nada
        &mut self,
        new_value: bool,
    ) -> Result<(), MsgLogicControlError> {
        self.external_peer_data.peer_choking = new_value;
        Ok(())
    }

    /// Funcion que actualiza si un peer está interesado en alguna de nuestras piezas
    ///
    pub fn update_peer_interested_field(
        &mut self,
        new_value: bool,
    ) -> Result<(), MsgLogicControlError> {
        self.external_peer_data.peer_interested = new_value;
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

    //UPDATE INFORMATION
    fn update_information_according_to_the_received_msg(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &mut TorrentStatus,
        received_msg: P2PMessage,
    ) -> Result<(), MsgLogicControlError> {
        match received_msg {
            P2PMessage::KeepAlive => Ok(()),
            P2PMessage::Choke => self.update_peer_choking_field(true),
            P2PMessage::Unchoke => self.update_peer_choking_field(false),
            P2PMessage::Have { piece_index } => self.update_server_peer_piece_status(
                usize::try_from(piece_index)
                    .map_err(|err| MsgLogicControlError::LookingForPieces(format!("{:?}", err)))?,
                PieceStatus::ValidAndAvailablePiece,
            ),
            P2PMessage::Bitfield { bitfield } => {
                self.update_peer_bitfield(torrent_file_data, bitfield)
            }
            P2PMessage::Piece {
                piece_index,
                beginning_byte_index,
                block,
            } => {
                let temp_path_name = torrent_file_data.get_torrent_representative_name();
                let piece_index = piece_index
                    .try_into()
                    .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?;
                self.store_block(
                    torrent_file_data,
                    torrent_status,
                    &temp_path_name,
                    piece_index,
                    beginning_byte_index,
                    block,
                )?;

                debug!(
                    "Nuevo estado de la pieza {}: {:?}",
                    piece_index,
                    torrent_status.pieces_availability[piece_index as usize] // (Miguel)REVISAR: Estaba indexado con un cero
                );
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
        torrent_status: &TorrentStatus,
    ) -> Result<(), MsgLogicControlError> {
        if self.peer_choking() {
            info!("Mensaje enviado: Interested");
            msg_sender::send_interested(&mut self.stream).map_err(|err| {
                if let MsgSenderError::WriteToTcpStream(_) = err {
                    MsgLogicControlError::ConectingWithPeer(format!("{:?}", err))
                } else {
                    MsgLogicControlError::SendingMessage(format!("{:?}", err))
                }
            })?;
        } else {
            msg_sender::send_request(
                &mut self.stream,
                torrent_file_data,
                torrent_status,
                piece_index,
            )
            .map_err(|err| {
                if let MsgSenderError::WriteToTcpStream(_) = err {
                    MsgLogicControlError::ConectingWithPeer(format!("{:?}", err))
                } else {
                    MsgLogicControlError::SendingMessage(format!("{:?}", err))
                }
            })?;
        }

        Ok(())
    }

    fn look_for_pieces(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &TorrentStatus,
    ) -> Result<(), MsgLogicControlError> {
        let piece_index = match torrent_status.look_for_a_missing_piece_index(&*self) {
            Some(piece_index) => {
                self.update_am_interested_field(true)?;
                piece_index
            }
            None => {
                self.update_am_interested_field(false)?;
                return Ok(());
            }
        };

        self.send_msg_according_to_peer_choking_field(
            piece_index,
            torrent_file_data,
            torrent_status,
        )?;

        Ok(())
    }

    // INTERACTION
    pub fn interact_with_peer(
        &mut self,
        torrent_file_data: &TorrentFileData,
        torrent_status: &mut TorrentStatus,
    ) -> Result<HandlerInteractionStatus, MsgLogicControlError> {
        loop {
            //RECIBO UN MENSAJE
            let received_msg =
                msg_receiver::receive_message(&mut self.stream).map_err(|error| {
                    if let MsgReceiverError::ReadingFromTcpStream(_) = error {
                        MsgLogicControlError::ConectingWithPeer(format!("{:?}", error))
                    } else {
                        MsgLogicControlError::ReceivingMessage(format!("{:?}", error))
                    }
                })?;
            log_info_msg(&received_msg);

            //ACTUALIZO MI INFORMACION SEGUN MENSAJE
            self.update_information_according_to_the_received_msg(
                torrent_file_data,
                torrent_status,
                received_msg,
            )?;

            //BUSCO SI TIENE UNA PIEZA QUE ME INTERESE Y ENVIO MENSAJE
            self.look_for_pieces(torrent_file_data, &*torrent_status)?;

            //VERIFICO SI DEBO CORTAR LA INTERACCION
            if !self.am_interested() {
                info!("Se busca un nuevo peer al cual pedirle piezas");
                return Ok(HandlerInteractionStatus::LookForAnotherPeer);
            } else if torrent_status
                .pieces_availability
                .iter()
                .any(|piece| *piece == PieceStatus::ValidAndAvailablePiece)
            {
                return Ok(HandlerInteractionStatus::FinishInteraction);
            }
        }
    }
}

#[cfg(test)]
mod test_client {
    use super::*;
    use std::{
        error::Error,
        fmt,
        io::ErrorKind,
        net::{SocketAddr, TcpListener, TcpStream},
        str::FromStr,
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
    };

    #[derive(PartialEq, Debug, Clone)]
    pub enum TestingError {
        ClientPeerFieldsInvalidAccess(String),
    }

    impl fmt::Display for TestingError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl Error for TestingError {}

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

    pub const DEFAULT_ADDR: &str = "127.0.0.1:8080";
    pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
    pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";
    pub const DEFAULT_INFO_HASH: [u8; 20] = [0; 20];

    fn create_default_client_peer_with_unused_server_peer() -> Result<
        (
            TrackerResponseData,
            TorrentStatus,
            TorrentFileData,
            LocalPeer,
        ),
        Box<dyn Error>,
    > {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;

        let handler = thread::spawn(move || listener.accept());

        let stream = TcpStream::connect(address)?;
        handler.join().unwrap()?; // feo pero para probar

        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
            peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
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
            pieces_availability: vec![PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: "resulting_filename.test".to_string(),
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
        let local_peer = LocalPeer {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            stream,
            external_peer_data: server_peer_data,
            role: PeerRole::Client,
        };
        Ok((tracker_response, torrent_status, torrent_file, local_peer))
    }

    fn create_default_client_peer_with_a_server_peer_that_has_the_whole_file() -> Result<
        (
            TrackerResponseData,
            TorrentStatus,
            TorrentFileData,
            LocalPeer,
        ),
        Box<dyn Error>,
    > {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;

        let handler = thread::spawn(move || listener.accept());

        let stream = TcpStream::connect(address)?;
        handler.join().unwrap()?; // feo pero para probar

        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
            peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
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
            pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: "resulting_filename.test".to_string(),
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
        let local_peer = LocalPeer {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            stream,
            external_peer_data: server_peer_data,
            role: PeerRole::Client,
        };
        Ok((tracker_response, torrent_status, torrent_file, local_peer))
    }

    fn create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces() -> Result<
        (
            TrackerResponseData,
            TorrentStatus,
            TorrentFileData,
            LocalPeer,
        ),
        Box<dyn Error>,
    > {
        let (listener, address) = try_bind_listener(STARTING_PORT)?;

        let handler = thread::spawn(move || listener.accept());

        let stream = TcpStream::connect(address)?;
        handler.join().unwrap()?; // feo pero para probar

        let server_peer = PeerDataFromTrackerResponse {
            peer_id: Some(DEFAULT_SERVER_PEER_ID.bytes().collect()),
            peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
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
            pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            target_files_data: TargetFilesData::SingleFile {
                file_name: "resulting_filename.test".to_string(),
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
            pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
            am_interested: false,
            am_choking: true,
            peer_choking: true,
            peer_interested: false,
        };
        let local_peer = LocalPeer {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            stream,
            external_peer_data: server_peer_data,
            role: PeerRole::Client,
        };
        Ok((tracker_response, torrent_status, torrent_file, local_peer))
    }

    mod test_generate_peer_data_from_handshake {
        use super::*;

        #[test]
        fn receive_a_message_that_is_not_a_handshake_error() -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let (tracker_response, _torrent_status, torrent_file_data, _) =
                create_default_client_peer_with_unused_server_peer()?;
            let message = P2PMessage::KeepAlive;

            assert!(generate_peer_data_from_handshake(
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
            let (tracker_response, _torrent_status, torrent_file_data, _) =
                create_default_client_peer_with_unused_server_peer()?;
            let message = P2PMessage::Handshake {
                protocol_str: "VitTorrent protocol".to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };

            assert!(generate_peer_data_from_handshake(
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
            let (tracker_response, _torrent_status, torrent_file_data, _) =
                create_default_client_peer_with_unused_server_peer()?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: [1; 20].to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };

            assert!(generate_peer_data_from_handshake(
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
            let (tracker_response, _torrent_status, torrent_file_data, _) =
                create_default_client_peer_with_unused_server_peer()?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: "-FA0001-000000000002".bytes().collect(),
            };

            assert!(generate_peer_data_from_handshake(
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

            let (mut tracker_response, _, torrent_file_data, _) =
                create_default_client_peer_with_unused_server_peer()?;
            println!("Si esto se imprime entonces ya salio de la conexion");

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
                pieces_availability: vec![PieceStatus::MissingPiece],
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };

            let received_peer_data = generate_peer_data_from_handshake(
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
            let (tracker_response, _torrent_status, torrent_file_data, _) =
                create_default_client_peer_with_unused_server_peer()?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };
            let expected_peer_data = PeerDataForP2PCommunication {
                pieces_availability: vec![PieceStatus::MissingPiece],
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };

            let received_peer_data = generate_peer_data_from_handshake(
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
            let (_tracker_response, _torrent_status, torrent_file_data, mut local_peer) =
                create_default_client_peer_with_unused_server_peer()?;
            let bitfield = vec![];

            assert!(local_peer
                .update_peer_bitfield(&torrent_file_data, bitfield)
                .is_err());

            assert_eq!(local_peer.external_peer_data.pieces_availability.len(), 1);
            Ok(())
        }

        #[test]
        fn update_peer_bitfield_with_more_pieces_and_spare_bits_set_error(
        ) -> Result<(), Box<dyn Error>> {
            let (_tracker_response, _torrent_status, torrent_file_data, mut local_peer) =
                create_default_client_peer_with_unused_server_peer()?;
            let bitfield = vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece,
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::ValidAndAvailablePiece,
            ];

            assert!(local_peer
                .update_peer_bitfield(&torrent_file_data, bitfield)
                .is_err());

            assert_eq!(local_peer.external_peer_data.pieces_availability.len(), 1);
            Ok(())
        }

        #[test]
        fn update_peer_bitfield_with_the_correct_amount_of_pieces_ok() -> Result<(), Box<dyn Error>>
        {
            let (_tracker_response, _torrent_status, torrent_file_data, mut local_peer) =
                create_default_client_peer_with_unused_server_peer()?;
            let bitfield = vec![PieceStatus::ValidAndAvailablePiece];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: vec![PieceStatus::MissingPiece],
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };
            local_peer.external_peer_data = peer_data;

            local_peer.update_peer_bitfield(&torrent_file_data, bitfield)?;

            assert_eq!(
                vec![PieceStatus::ValidAndAvailablePiece],
                local_peer.external_peer_data.pieces_availability
            );
            Ok(())
        }

        #[test]
        fn update_peer_bitfield_with_more_pieces_and_spare_bits_not_set_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (_tracker_response, _torrent_status, torrent_file_data, mut local_peer) =
                create_default_client_peer_with_unused_server_peer()?;
            let bitfield = vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
            ];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: vec![PieceStatus::MissingPiece],
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
                peer_interested: false,
            };
            local_peer.external_peer_data = peer_data;

            local_peer.update_peer_bitfield(&torrent_file_data, bitfield)?;

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
            let (_, _, _torrent_file_data, mut local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces()?;

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
            let (_, _, _, mut local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces()?;

            assert!(local_peer
                .update_server_peer_piece_status(piece_index, PieceStatus::ValidAndAvailablePiece,)
                .is_err());

            Ok(())
        }
    }

    mod test_store_block {
        use std::fs;

        use crate::torrent::client::peers_comunication::msg_logic_control::BLOCK_BYTES;

        use super::*;

        #[test]
        fn the_received_block_is_smaller_than_expected_error() -> Result<(), Box<dyn Error>> {
            let (_tracker_response, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block = vec![];
            let path = "test_client/store_block_1".to_string();

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] Block length is not as expected".to_string()
                )),
                local_peer.store_block(
                    &torrent_file_data,
                    &mut torrent_status,
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
            let (_tracker_response, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block = [0; BLOCK_BYTES as usize + 1].to_vec();
            let path = "test_client/store_block_2".to_string();

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] Block length is not as expected".to_string()
                )),
                local_peer.store_block(
                    &torrent_file_data,
                    &mut torrent_status,
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
            let (_tracker_response, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 2;
            let beginning_byte_index = 0;
            let block = [0; BLOCK_BYTES as usize].to_vec();
            let path = "test_client/store_block_3".to_string();

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] The received piece index is invalid.".to_string(),
                )),
                local_peer.store_block(
                    &torrent_file_data,
                    &mut torrent_status,
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
            let (_tracker_response, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block = [0; 16384].to_vec();
            let path = "test_client/store_block_4".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block,
            )?;

            if let Some(PieceStatus::PartiallyDownloaded { downloaded_bytes }) =
                torrent_status.pieces_availability.get(piece_index as usize)
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
            let (_tracker_response, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 1;
            let beginning_byte_index = 0;
            let block = [0; 6000].to_vec();
            let path = "test_client/store_block_5".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block,
            )?;

            if let Some(piece_status) = torrent_status.pieces_availability.get(piece_index as usize)
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
        fn the_client_peer_receives_a_piece_that_already_own_error() -> Result<(), Box<dyn Error>> {
            let (_tracker_response, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 1;
            let mut beginning_byte_index = 0;
            let block = [0; 6000].to_vec();
            let path = "test_client/store_block_6".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block.clone(),
            )?;
            beginning_byte_index = 6000;

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] The client peer has already completed that piece."
                        .to_string(),
                )),
                local_peer.store_block(
                    &torrent_file_data,
                    &mut torrent_status,
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
            let (_, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 0;
            let mut beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let path = "test_client/store_block_7".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_1,
            )?;
            beginning_byte_index = 16384;
            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_2,
            )?;

            if let Some(PieceStatus::PartiallyDownloaded { downloaded_bytes }) =
                torrent_status.pieces_availability.get(piece_index as usize)
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
            let (_tracker_response, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 0;
            let mut beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let block_3 = [0; 34000 - (2 * 16384)].to_vec();
            let path = "test_client/store_block_8".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_1,
            )?;
            beginning_byte_index = 16384;
            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_2,
            )?;
            beginning_byte_index = 16384 * 2;
            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_3,
            )?;

            if let Some(piece_status) = torrent_status.pieces_availability.get(piece_index as usize)
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
            let (_tracker_response, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let path = "test_client/store_block_9".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_1,
            )?;

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] The beginning byte index is incorrect.".to_string(),
                )),
                local_peer.store_block(
                    &torrent_file_data,
                    &mut torrent_status,
                    &path,
                    piece_index,
                    beginning_byte_index,
                    block_2
                )
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn the_client_peer_receives_three_blocks_and_updates_downloaded_data_ok(
        ) -> Result<(), Box<dyn Error>> {
            let (_tracker_response, mut torrent_status, torrent_file_data, local_peer) =
                create_default_client_peer_with_a_server_peer_that_has_the_whole_file()?;
            let piece_index = 0;
            let mut beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let block_3 = [0; 34000 - (2 * 16384)].to_vec();
            let path = "test_client/store_block_10".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            assert_eq!(0, torrent_status.downloaded);
            assert_eq!(40000, torrent_status.left);

            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_1,
            )?;
            assert_eq!(16384, torrent_status.downloaded);
            assert_eq!(40000 - 16384, torrent_status.left);

            beginning_byte_index = 16384;

            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_2,
            )?;
            assert_eq!(16384 * 2, torrent_status.downloaded);
            assert_eq!(40000 - 16384 * 2, torrent_status.left);

            beginning_byte_index = 16384 * 2;

            local_peer.store_block(
                &torrent_file_data,
                &mut torrent_status,
                &path,
                piece_index,
                beginning_byte_index,
                block_3,
            )?;
            assert_eq!(34000, torrent_status.downloaded);
            assert_eq!(40000 - 34000, torrent_status.left);

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }
    }
}
