//! # Modulo de manejo general de la estructura principal: Client
//! Este modulo contiene las funciones encargadas del comportamiento general
//! de nuestro cliente como peer de tipo leecher.

use crate::torrent::{
    client::tracker_comunication::http_handler::HttpHandler,
    data::{
        data_of_download::DataOfDownload,
        medatada_analyzer::{read_torrent_file_to_dic, MetadataError},
        peer_data_for_communication::PeerDataForP2PCommunication,
        torrent_file_data::{TorrentError, TorrentFileData},
        tracker_response_data::{ResponseError, TrackerResponseData},
    },
    parsers::p2p::{
        constants::PSTR_STRING_HANDSHAKE,
        message::{P2PMessage, PieceStatus},
    },
};

use super::{
    block_handler,
    peers_comunication::msg_logic_control::{MsgLogicControlError, BLOCK_BYTES},
    tracker_comunication::http_handler::ErrorMsgHttp,
};

extern crate rand;

use log::{debug, error, info, trace};
use rand::{distributions::Alphanumeric, Rng};

use std::{error::Error, fmt, fs};

const SIZE_PEER_ID: usize = 12;
const INIT_PEER_ID: &str = "-FA0000-";

type ResultClient<T> = Result<T, ClientError>;

#[derive(PartialEq, Debug, Clone)]
/// Struct que tiene por comportamiento todo el manejo general de actualizacion importante de datos, almacenamiento de los mismos y ejecución de metodos importantes para la comunicación con peers durante la ejecución del programa a modo de leecher.
pub struct Client {
    pub peer_id: Vec<u8>,
    pub info_hash: Vec<u8>,

    pub data_of_download: DataOfDownload,
    pub torrent_file: TorrentFileData,
    pub tracker_response: Option<TrackerResponseData>,
    pub list_of_peers_data_for_communication: Option<Vec<PeerDataForP2PCommunication>>,
}

#[derive(PartialEq, Debug)]
// Representa posibles errores durante la ejecucion de alguna de sus funcionalidades
pub enum ClientError {
    File(MetadataError),
    HttpCreation(ErrorMsgHttp),
    ConectionError(ErrorMsgHttp),
    TorrentCreation(TorrentError),
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
pub fn init_communication(torrent: TorrentFileData) -> ResultClient<TrackerResponseData> {
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

fn update_list_of_peers_data_for_communication(
    list_of_peers_data_for_communication: &mut [PeerDataForP2PCommunication],
    bitfield: Vec<PieceStatus>,
    server_peer_index: usize,
) -> Result<(), MsgLogicControlError> {
    let peer_data = list_of_peers_data_for_communication.get_mut(server_peer_index);
    match peer_data {
        Some(peer_data) => {
            peer_data.pieces_availability = Some(bitfield);
            Ok(())
        }
        None => Err(MsgLogicControlError::UpdatingBitfield(
            "[MsgLogicControlError] Couldn`t find a server peer on the given index".to_string(),
        )),
    }
}

impl Client {
    /// Funcion que interpreta toda la info del .torrent, se comunica con el
    /// tracker correspondiente y almacena todos los datos importantes para
    /// su uso posterior en comunicacion con peers, devolviendo así
    /// una instancia de la estructura lista para ello.
    ///
    pub fn new(path_file: &str) -> ResultClient<Self> {
        trace!("Genero peer_id");
        let peer_id = generate_peer_id();
        let torrent_file = create_torrent(path_file)?;
        trace!("TorrentFileData creado y almacenado dentro del Client");
        let info_hash = torrent_file.get_info_hash();
        let torrent_size = torrent_file.get_total_size() as u64;
        let data_of_download = DataOfDownload::new(torrent_size, torrent_file.total_amount_pieces);
        Ok(Client {
            peer_id,
            torrent_file,
            data_of_download,
            info_hash,
            tracker_response: None,
            list_of_peers_data_for_communication: None,
        })
    }

    /// Funcion que realiza toda la comunicación con el tracker, interpreta su
    /// respuesta y almacena la info importante de la misma
    ///
    pub fn init_communication(&mut self) -> ResultClient<()> {
        match init_communication(self.torrent_file.clone()) {
            Ok(response) => self.tracker_response = Some(response),
            Err(error) => return Err(error),
        };
        trace!("TrackerResponseData creado y almacenado dentro del Client");
        Ok(())
    }

    //HANDSHAKE
    fn has_expected_peer_id(&self, server_peer_id: &[u8], server_peer_index: usize) -> bool {
        if let Some(tracker_response) = &self.tracker_response {
            match tracker_response.peers.get(server_peer_index) {
                Some(tracker_response_peer_data) => {
                    if let Some(tracker_response_peer_id) = &tracker_response_peer_data.peer_id {
                        tracker_response_peer_id == server_peer_id
                    } else {
                        true
                    }
                }
                None => false,
            }
        } else {
            true
        }
    }

    fn check_handshake(
        &self,
        server_protocol_str: String,
        server_info_hash: Vec<u8>,
        server_peer_id: &[u8],
        server_peer_index: usize,
    ) -> Result<(), MsgLogicControlError> {
        if (server_protocol_str != PSTR_STRING_HANDSHAKE)
            || (server_info_hash != self.torrent_file.info_hash)
            || !self.has_expected_peer_id(server_peer_id, server_peer_index)
        {
            return Err(MsgLogicControlError::CheckingAndSavingHandshake(
                "[MsgLogicControlError] The received handshake hasn`t got the expected fields."
                    .to_string(),
            ));
        }
        Ok(())
    }

    fn save_handshake_data(&mut self, server_peer_id: Vec<u8>) {
        let new_peer = PeerDataForP2PCommunication::new(server_peer_id);

        match &mut self.list_of_peers_data_for_communication {
            Some(list_of_peers_data_for_communication) => {
                list_of_peers_data_for_communication.push(new_peer);
            }
            None => {
                self.list_of_peers_data_for_communication = Some(vec![new_peer]);
            }
        };
    }

    /// Funcion que realiza la verificacion de un mensaje recibido de tipo
    /// Handshake y almacena su info importante
    ///
    pub fn check_and_save_handshake_data(
        &mut self,
        message: P2PMessage,
        server_peer_index: usize,
    ) -> Result<(), MsgLogicControlError> {
        if let P2PMessage::Handshake {
            protocol_str: server_protocol_str,
            info_hash: server_info_hash,
            peer_id: server_peer_id,
        } = message
        {
            self.check_handshake(
                server_protocol_str,
                server_info_hash,
                &server_peer_id,
                server_peer_index,
            )?;
            self.save_handshake_data(server_peer_id);
            Ok(())
        } else {
            Err(MsgLogicControlError::CheckingAndSavingHandshake(
                "[MsgLogicControlError] The received messagge is not a handshake.".to_string(),
            ))
        }
    }

    //BITFIELD
    fn is_any_spare_bit_set(&self, bitfield: &[PieceStatus]) -> bool {
        return bitfield
            .iter()
            .skip(self.torrent_file.total_amount_pieces)
            .any(|piece_status| *piece_status == PieceStatus::ValidAndAvailablePiece);
    }

    fn check_bitfield(&self, bitfield: &mut [PieceStatus]) -> Result<(), MsgLogicControlError> {
        if bitfield.len() < self.torrent_file.total_amount_pieces {
            return Err(MsgLogicControlError::UpdatingBitfield(
                "[MsgLogicControlError] The bitfield length is incorrect.".to_string(),
            ));
        }

        if self.is_any_spare_bit_set(bitfield) {
            return Err(MsgLogicControlError::UpdatingBitfield(
                "[MsgLogicControlError] Some of the spare bits are set.".to_string(),
            ));
        }
        Ok(())
    }

    fn check_and_truncate_bitfield_according_to_total_amount_of_pieces(
        &self,
        bitfield: &mut Vec<PieceStatus>,
    ) -> Result<(), MsgLogicControlError> {
        self.check_bitfield(bitfield)?;
        bitfield.truncate(self.torrent_file.total_amount_pieces);
        Ok(())
    }

    /// Funcion que actualiza la representación de bitfield de un peer dado
    /// por su indice
    ///
    pub fn update_peer_bitfield(
        &mut self,
        mut bitfield: Vec<PieceStatus>,
        server_peer_index: usize,
    ) -> Result<(), MsgLogicControlError> {
        self.check_and_truncate_bitfield_according_to_total_amount_of_pieces(&mut bitfield)?;
        match &mut self.list_of_peers_data_for_communication {
            Some(list_of_peers_data_for_communication) => {
                update_list_of_peers_data_for_communication(
                    list_of_peers_data_for_communication,
                    bitfield,
                    server_peer_index,
                )
            }
            None => Err(MsgLogicControlError::UpdatingBitfield(
                "[MsgLogicControlError] Server peers list invalid access".to_string(),
            )),
        }
    }

    // HAVE
    /// Funcion que actualiza la representación de bitfield de un peer dado
    /// por su indice (A diferencia de [update_peer_bitfield()], esta funcion
    /// actualiza solo el estado de UNA pieza, esto es causado por
    /// la recepcion de un mensaje P2P de tipo Have)
    ///
    pub fn update_server_peer_piece_status(
        &mut self,
        server_peer_index: usize,
        piece_index: u32,
        new_status: PieceStatus,
    ) -> Result<(), MsgLogicControlError> {
        if let Some(list_of_peers_data_for_communication) =
            &mut self.list_of_peers_data_for_communication
        {
            if let Some(peer_data) = list_of_peers_data_for_communication.get_mut(server_peer_index)
            {
                if let Some(pieces_availability) = &mut peer_data.pieces_availability {
                    if let Some(piece_status) =
                        pieces_availability.get_mut(usize::try_from(piece_index).map_err(
                            |err| MsgLogicControlError::UpdatingPieceStatus(format!("{:?}", err)),
                        )?)
                    {
                        *piece_status = new_status;
                        return Ok(());
                    }
                    return Err(MsgLogicControlError::UpdatingPieceStatus(
                        "[MsgLogicControlError] Invalid piece index.".to_string(),
                    ));
                }
            }
        }

        Err(MsgLogicControlError::UpdatingPieceStatus(
            "[MsgLogicControlError] Client peer invalid access.".to_string(),
        ))
    }

    //PIECE
    fn check_piece_index_and_beginning_byte_index(
        &self,
        piece_index: u32,
        beginning_byte_index: u32,
    ) -> Result<(), MsgLogicControlError> {
        match self.data_of_download.pieces_availability.get(
            usize::try_from(piece_index)
                .map_err(|err| MsgLogicControlError::LookingForPieces(format!("{:?}", err)))?,
        ) {
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
        piece_index: u32,
        beginning_byte_index: u32,
        block: &[u8],
    ) -> Result<(), MsgLogicControlError> {
        let expected_amount_of_bytes = self
            .calculate_amount_of_bytes(piece_index, beginning_byte_index)?
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
        piece_index: u32,
        beginning_byte_index: u32,
        block: &[u8],
    ) -> Result<(), MsgLogicControlError> {
        self.check_piece_index_and_beginning_byte_index(piece_index, beginning_byte_index)?;
        self.check_block_lenght(piece_index, beginning_byte_index, block)?;
        Ok(())
    }

    fn set_up_directory(&mut self, path: &str) -> Result<(), MsgLogicControlError> {
        if self
            .data_of_download
            .pieces_availability
            .iter()
            .all(|piece_status| *piece_status == PieceStatus::MissingPiece)
        {
            info!("Creo un directorio para guardar piezas");
            let _result = fs::remove_dir_all(format!("temp/{}", path));
            fs::create_dir(format!("temp/{}", path))
                .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?;
        }
        Ok(())
    }

    fn update_client_peer_piece_status(
        &mut self,
        piece_index: u32,
        beginning_byte_index: u32,
        amount_of_bytes: u32,
    ) -> Result<(), MsgLogicControlError> {
        let piece_lenght = self.calculate_piece_lenght(piece_index)?;

        if let Some(piece_status) = self
            .data_of_download
            .pieces_availability
            .get_mut(piece_index as usize)
        {
            match piece_status {
                PieceStatus::MissingPiece => {
                    if piece_lenght == amount_of_bytes {
                        *piece_status = PieceStatus::ValidAndAvailablePiece;
                    } else {
                        *piece_status = PieceStatus::PartiallyDownloaded {
                            downloaded_bytes: amount_of_bytes,
                        };
                    }
                }
                PieceStatus::PartiallyDownloaded { downloaded_bytes } => {
                    let remaining_bytes = piece_lenght - *downloaded_bytes - amount_of_bytes;
                    if remaining_bytes == 0 {
                        *piece_status = PieceStatus::ValidAndAvailablePiece;
                    } else {
                        *piece_status = PieceStatus::PartiallyDownloaded {
                            downloaded_bytes: beginning_byte_index + amount_of_bytes,
                        };
                    }
                }
                PieceStatus::ValidAndAvailablePiece => {
                    return Err(MsgLogicControlError::StoringBlock(
                        "[MsgLogicControlError] The client peer has already completed that piece."
                            .to_string(),
                    ))
                }
            }
        };
        Ok(())
    }

    fn update_client_peer_data_of_download(self: &mut Client, amount_of_bytes: u64) {
        self.data_of_download.downloaded += amount_of_bytes;
        self.data_of_download.left -= amount_of_bytes;
    }

    /// Funcion encargada de realizar toda la logica de guardado de
    /// un bloque en disco y actualizacion correspondiente de
    /// mi propio bitfield y el estado de la descarga.
    /// Si se completa una pieza tras el guardado, se verifica la
    /// misma por medio de su SHA1 y el que venia como correspondiente
    /// a dicha pieza en el .torrent
    ///
    pub fn store_block(
        &mut self,
        piece_index: u32,
        beginning_byte_index: u32,
        block: Vec<u8>,
        path: &str,
    ) -> Result<(), MsgLogicControlError> {
        self.check_store_block(piece_index, beginning_byte_index, &block)?;
        self.set_up_directory(path)?;
        block_handler::store_block(&block, piece_index, path)
            .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?;

        self.update_client_peer_piece_status(
            piece_index,
            beginning_byte_index,
            u32::try_from(block.len())
                .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?,
        )?;
        self.update_client_peer_data_of_download(
            u64::try_from(block.len())
                .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?,
        );

        if self.data_of_download.is_a_valid_and_available_piece(
            usize::try_from(piece_index)
                .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?,
        ) {
            info!("Se completó la pieza {}.", piece_index);
            info!("Verifico el hash SHA1 de la pieza descargada.");
            block_handler::check_sha1_piece(self, piece_index, path)
                .map_err(|err| MsgLogicControlError::StoringBlock(format!("{:?}", err)))?;
        }
        Ok(())
    }

    // UPDATING FIELDS
    /// Funcion que actualiza si el cliente está interesado en una pieza
    /// de un peer dado por su indice.
    ///
    pub fn update_am_interested_field(
        &mut self,
        server_peer_index: usize,
        new_value: bool,
    ) -> Result<(), MsgLogicControlError> {
        if let Some(list_of_peers_data_for_communication) =
            &mut self.list_of_peers_data_for_communication
        {
            if let Some(server_peer_data) =
                list_of_peers_data_for_communication.get_mut(server_peer_index)
            {
                server_peer_data.am_interested = new_value;
            }
            Ok(())
        } else {
            Err(MsgLogicControlError::UpdatingFields(
                "[MsgLogicControlError] Server peers list invalid access".to_string(),
            ))
        }
    }

    /// Funcion que actualiza si un peer me tiene chokeado a mi cliente
    ///
    pub fn update_peer_choking_field(
        &mut self,
        server_peer_index: usize,
        new_value: bool,
    ) -> Result<(), MsgLogicControlError> {
        if let Some(list_of_peers_data_for_communication) =
            &mut self.list_of_peers_data_for_communication
        {
            if let Some(server_peer_data) =
                list_of_peers_data_for_communication.get_mut(server_peer_index)
            {
                server_peer_data.peer_choking = new_value;
            }
            Ok(())
        } else {
            Err(MsgLogicControlError::UpdatingFields(
                "[MsgLogicControlError] Server peers list invalid access".to_string(),
            ))
        }
    }

    /// Funcion que actualiza si mi cliente tiene chokeado a un peer especifico
    ///
    pub fn update_am_choking_field(
        &mut self,
        server_peer_index: usize,
        new_value: bool,
    ) -> Result<(), MsgLogicControlError> {
        if let Some(list_of_peers_data_for_communication) =
            &mut self.list_of_peers_data_for_communication
        {
            if let Some(server_peer_data) =
                list_of_peers_data_for_communication.get_mut(server_peer_index)
            {
                server_peer_data.am_choking = new_value;
            }
            Ok(())
        } else {
            Err(MsgLogicControlError::UpdatingFields(
                "[MsgLogicControlError] Server peers list invalid access".to_string(),
            ))
        }
    }

    // ASK FOR INFORMATION
    /// Funcion que revisa si tal peer me tiene chokeado a mi
    ///
    pub fn peer_choking(&self, server_peer_index: usize) -> bool {
        if let Some(list_of_peers_data_for_communication) =
            &self.list_of_peers_data_for_communication
        {
            if let Some(server_peer_data) =
                list_of_peers_data_for_communication.get(server_peer_index)
            {
                return server_peer_data.peer_choking;
            }
        }
        true
    }

    /// Funcion que revisa si mi cliente esta interesado en un peer especifico
    ///
    pub fn am_interested(&self, server_peer_index: usize) -> bool {
        if let Some(list_of_peers_data_for_communication) =
            &self.list_of_peers_data_for_communication
        {
            if let Some(server_peer_data) =
                list_of_peers_data_for_communication.get(server_peer_index)
            {
                return server_peer_data.am_interested;
            }
        }
        false
    }

    //LOOK FOR PIECES
    fn server_peer_has_a_valid_and_available_piece_on_position(
        &self,
        server_peer_index: usize,
        position: usize,
    ) -> bool {
        if let Some(list_of_peers_data_for_communication) =
            &self.list_of_peers_data_for_communication
        {
            if let Some(server_peer_data) =
                list_of_peers_data_for_communication.get(server_peer_index)
            {
                if let Some(pieces_availability) = &server_peer_data.pieces_availability {
                    return pieces_availability[position] == PieceStatus::ValidAndAvailablePiece;
                }
            }
        }

        false
    }

    /// Funcion que busca una nueva pieza que quiera pedir posteriormente, y
    /// devuelve su indice
    ///
    pub fn look_for_a_missing_piece_index(&self, server_peer_index: usize) -> Option<usize> {
        let (piece_index, _piece_status) = self
            .data_of_download
            .pieces_availability
            .iter()
            .enumerate()
            .find(|(piece_index, piece_status)| {
                (**piece_status != PieceStatus::ValidAndAvailablePiece)
                    && self.server_peer_has_a_valid_and_available_piece_on_position(
                        server_peer_index,
                        *piece_index,
                    )
            })?;
        Some(piece_index)
    }

    /// Funcion que calcula el byte inicial desde el cual
    /// se deberia pedir el siguiente bloque de una pieza
    ///
    pub fn calculate_beginning_byte_index(
        &self,
        piece_index: u32,
    ) -> Result<u32, MsgLogicControlError> {
        match self
        .data_of_download
        .pieces_availability
        .get(usize::try_from(piece_index)
        .map_err(|err| MsgLogicControlError::LookingForPieces(format!("{:?}", err)))?)
        {
            Some(PieceStatus::PartiallyDownloaded { downloaded_bytes }) => Ok(*downloaded_bytes),
            Some(PieceStatus::MissingPiece) => Ok(0),
            _ => Err(MsgLogicControlError::LookingForPieces(
                "[MsgLogicControlError] Invalid piece index given in order to calculate beggining byte index."
                    .to_string(),
            )),
        }
    }

    fn is_last_piece_index(&self, piece_index: u32) -> bool {
        self.data_of_download.pieces_availability.len() - 1 == piece_index as usize
    }

    /// Funcion que calcula la longitud de las piezas (NO bloques) a ser pedidas
    ///
    pub fn calculate_piece_lenght(&self, piece_index: u32) -> Result<u32, MsgLogicControlError> {
        if self.is_last_piece_index(piece_index) {
            let std_piece_lenght = self.torrent_file.piece_length;
            let total_amount_pieces = i64::try_from(self.torrent_file.total_amount_pieces)
                .map_err(|err| {
                    MsgLogicControlError::CalculatingPieceLenght(format!("{:?}", err))
                })?;
            let total_size = self.torrent_file.total_size;

            u32::try_from(total_size - (std_piece_lenght * (total_amount_pieces - 1)))
                .map_err(|err| MsgLogicControlError::CalculatingPieceLenght(format!("{:?}", err)))
        } else {
            u32::try_from(self.torrent_file.piece_length)
                .map_err(|err| MsgLogicControlError::CalculatingPieceLenght(format!("{:?}", err)))
        }
    }

    /// Funcion que calcula la cantidad de bytes adecuada a pedir
    /// posteriormente a un peer
    ///
    pub fn calculate_amount_of_bytes(
        &self,
        piece_index: u32,
        beginning_byte_index: u32,
    ) -> Result<u32, MsgLogicControlError> {
        let piece_lenght = self.calculate_piece_lenght(piece_index)?;

        let remaining_bytes = piece_lenght - beginning_byte_index;
        if remaining_bytes <= BLOCK_BYTES {
            Ok(remaining_bytes)
        } else {
            Ok(BLOCK_BYTES)
        }
    }
}

#[cfg(test)]
mod test_client {
    use super::*;
    use std::{error::Error, net::SocketAddr, str::FromStr};

    use crate::torrent::{
        data::{
            data_of_download::{DataOfDownload, StateOfDownload},
            torrent_file_data::TorrentFileData,
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

    pub const DEFAULT_ADDR: &str = "127.0.0.1:8080";
    pub const DEFAULT_CLIENT_PEER_ID: &str = "-FA0001-000000000000";
    pub const DEFAULT_SERVER_PEER_ID: &str = "-FA0001-000000000001";
    pub const DEFAULT_INFO_HASH: [u8; 20] = [0; 20];

    fn create_default_client_peer_with_no_server_peers() -> Result<Client, Box<dyn Error>> {
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
        let data_of_download = DataOfDownload {
            uploaded: 0,
            downloaded: 0,
            left: 16,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            is_single_file: true,
            name: "resulting_filename.test".to_string(),
            pieces: vec![],
            path: vec![],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 16,
            total_amount_pieces: 1,
            total_size: 16,
        };
        Ok(Client {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            data_of_download,
            torrent_file,
            tracker_response: Some(tracker_response),
            list_of_peers_data_for_communication: None,
        })
    }

    fn create_default_client_peer_with_a_server_peer_that_has_the_hole_file(
    ) -> Result<Client, Box<dyn Error>> {
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
        let data_of_download = DataOfDownload {
            uploaded: 0,
            downloaded: 0,
            left: 40000,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            is_single_file: true,
            name: "resulting_filename.test".to_string(),
            pieces: vec![
                46, 101, 88, 42, 242, 153, 87, 30, 42, 117, 240, 135, 191, 37, 12, 42, 175, 156,
                136, 214, 95, 100, 198, 139, 237, 56, 161, 225, 113, 168, 52, 228, 26, 36, 103,
                150, 103, 76, 233, 34,
            ],
            path: vec![],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 34000,
            total_amount_pieces: 2,
            total_size: 40000,
            //1º pieza -> 34000 bytes
            //2º pieza ->  6000 bytes
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            pieces_availability: Some(vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::ValidAndAvailablePiece,
            ]),
            am_interested: false,
            am_choking: true,
            peer_choking: true,
        };
        let list_of_peers_data_for_communication = Some(vec![server_peer_data]);
        Ok(Client {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            data_of_download,
            torrent_file,
            tracker_response: Some(tracker_response),
            list_of_peers_data_for_communication,
        })
    }

    fn create_default_client_peer_with_a_server_peer_that_has_just_one_valid_piece(
    ) -> Result<Client, Box<dyn Error>> {
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
        let data_of_download = DataOfDownload {
            uploaded: 0,
            downloaded: 0,
            left: 16,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            is_single_file: true,
            name: "resulting_filename.test".to_string(),
            pieces: vec![],
            path: vec![],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 16,
            total_amount_pieces: 2,
            total_size: 32,
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            pieces_availability: Some(vec![
                PieceStatus::MissingPiece,
                PieceStatus::ValidAndAvailablePiece,
            ]),
            am_interested: false,
            am_choking: true,
            peer_choking: true,
        };
        let list_of_peers_data_for_communication = Some(vec![server_peer_data]);
        Ok(Client {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            data_of_download,
            torrent_file,
            tracker_response: Some(tracker_response),
            list_of_peers_data_for_communication,
        })
    }

    fn create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces(
    ) -> Result<Client, Box<dyn Error>> {
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
        let data_of_download = DataOfDownload {
            uploaded: 0,
            downloaded: 0,
            left: 16,
            event: StateOfDownload::Started,
            pieces_availability: vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece],
        };
        let torrent_file = TorrentFileData {
            is_single_file: true,
            name: "resulting_filename.test".to_string(),
            pieces: vec![],
            path: vec![],
            url_tracker_main: "tracker_main.com".to_string(),
            url_tracker_list: vec![],
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            piece_length: 16,
            total_amount_pieces: 2,
            total_size: 32,
        };
        let server_peer_data = PeerDataForP2PCommunication {
            peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            pieces_availability: Some(vec![PieceStatus::MissingPiece, PieceStatus::MissingPiece]),
            am_interested: false,
            am_choking: true,
            peer_choking: true,
        };
        let list_of_peers_data_for_communication = Some(vec![server_peer_data]);
        Ok(Client {
            peer_id: DEFAULT_CLIENT_PEER_ID.bytes().collect(),
            info_hash: DEFAULT_INFO_HASH.to_vec(),
            data_of_download,
            torrent_file,
            tracker_response: Some(tracker_response),
            list_of_peers_data_for_communication,
        })
    }

    mod test_check_and_save_handshake_data {
        use super::*;

        #[test]
        fn receive_a_message_that_is_not_a_handshake_error() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::KeepAlive;

            assert!(client_peer
                .check_and_save_handshake_data(message, server_piece_index)
                .is_err());

            Ok(())
        }

        #[test]
        fn receive_a_handshake_with_an_incorrect_protocol_str_error() -> Result<(), Box<dyn Error>>
        {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::Handshake {
                protocol_str: "VitTorrent protocol".to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };

            assert!(client_peer
                .check_and_save_handshake_data(message, server_piece_index)
                .is_err());

            Ok(())
        }

        #[test]
        fn receive_a_handshake_with_an_incorrect_info_hash_error() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: [1; 20].to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };

            assert!(client_peer
                .check_and_save_handshake_data(message, server_piece_index)
                .is_err());

            Ok(())
        }

        #[test]
        fn receive_a_handshake_with_an_incorrect_peer_id_error() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: "-FA0001-000000000002".bytes().collect(),
            };

            assert!(client_peer
                .check_and_save_handshake_data(message, server_piece_index)
                .is_err());

            Ok(())
        }

        #[test]
        fn client_that_has_no_peer_ids_to_check_receive_a_valid_handshake_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;

            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            //MODIFICO EL CLIENTE PARA QUE NO TENGA LOS PEER_ID DE LOS SERVER PEER
            if let Some(tracker_response) = &mut client_peer.tracker_response {
                tracker_response.peers = vec![PeerDataFromTrackerResponse {
                    peer_id: None,
                    peer_address: SocketAddr::from_str(DEFAULT_ADDR)?,
                }];
            }

            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };
            let expected_peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };

            client_peer.check_and_save_handshake_data(message, server_piece_index)?;

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                assert_eq!(vec![expected_peer_data], peer_data_list);
                assert_eq!(1, peer_data_list.len());
                return Ok(());
            }

            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn client_that_has_peer_ids_to_check_receive_a_valid_handshake_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let message = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: DEFAULT_INFO_HASH.to_vec(),
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
            };
            let expected_peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };

            client_peer.check_and_save_handshake_data(message, server_peer_index)?;

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                assert_eq!(vec![expected_peer_data], peer_data_list);
                assert_eq!(1, peer_data_list.len());
                return Ok(());
            }
            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }
    }

    mod test_update_peer_bitfield {
        use super::*;

        #[test]
        fn update_peer_bitfield_with_less_pieces_error() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let bitfield = vec![];

            let peer_data = PeerDataForP2PCommunication {
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                pieces_availability: None,
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };
            let peer_data_list = vec![peer_data];
            client_peer.list_of_peers_data_for_communication = Some(peer_data_list);

            assert!(client_peer
                .update_peer_bitfield(bitfield, server_piece_index)
                .is_err());

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                if let Some(server_peer_data) = peer_data_list.get(server_piece_index) {
                    assert!(server_peer_data.pieces_availability.is_none());
                    return Ok(());
                }
            }

            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn update_peer_bitfield_with_more_pieces_and_spare_bits_set_error(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let bitfield = vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece,
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::ValidAndAvailablePiece,
            ];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };
            let peer_data_list = vec![peer_data];
            client_peer.list_of_peers_data_for_communication = Some(peer_data_list);

            assert!(client_peer
                .update_peer_bitfield(bitfield, server_piece_index)
                .is_err());

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                if let Some(server_peer_data) = peer_data_list.get(server_piece_index) {
                    assert!(server_peer_data.pieces_availability.is_none());
                    return Ok(());
                }
            }

            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn update_peer_bitfield_with_the_correct_amount_of_pieces_ok() -> Result<(), Box<dyn Error>>
        {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let bitfield = vec![PieceStatus::ValidAndAvailablePiece];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };
            let peer_data_list = vec![peer_data];
            client_peer.list_of_peers_data_for_communication = Some(peer_data_list);

            client_peer.update_peer_bitfield(bitfield, server_piece_index)?;

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                if let Some(server_peer_data) = peer_data_list.get(server_piece_index) {
                    if let Some(piece_availability) = &server_peer_data.pieces_availability {
                        assert_eq!(
                            vec![PieceStatus::ValidAndAvailablePiece],
                            *piece_availability
                        )
                    }
                    return Ok(());
                }
            }
            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn update_peer_bitfield_with_more_pieces_and_spare_bits_not_set_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer = create_default_client_peer_with_no_server_peers()?;
            let bitfield = vec![
                PieceStatus::ValidAndAvailablePiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
                PieceStatus::MissingPiece,
            ];

            let peer_data = PeerDataForP2PCommunication {
                pieces_availability: None,
                peer_id: DEFAULT_SERVER_PEER_ID.bytes().collect(),
                am_interested: false,
                am_choking: true,
                peer_choking: true,
            };
            let peer_data_list = vec![peer_data];
            client_peer.list_of_peers_data_for_communication = Some(peer_data_list);

            client_peer.update_peer_bitfield(bitfield, server_piece_index)?;

            if let Some(peer_data_list) = client_peer.list_of_peers_data_for_communication {
                if let Some(server_peer_data) = peer_data_list.get(server_piece_index) {
                    if let Some(piece_availability) = &server_peer_data.pieces_availability {
                        assert_eq!(
                            vec![PieceStatus::ValidAndAvailablePiece],
                            *piece_availability
                        );
                        return Ok(());
                    }
                }
            }
            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }
    }

    mod test_look_for_a_missing_piece_index {
        use super::*;

        #[test]
        fn the_server_peer_has_a_valid_and_available_piece_in_the_position_zero_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;

            assert_eq!(
                Some(0),
                client_peer.look_for_a_missing_piece_index(server_piece_index)
            );
            Ok(())
        }

        #[test]
        fn the_server_peer_has_a_valid_and_available_piece_in_the_position_one_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let client_peer =
                create_default_client_peer_with_a_server_peer_that_has_just_one_valid_piece()?;

            assert_eq!(
                Some(1),
                client_peer.look_for_a_missing_piece_index(server_piece_index)
            );
            Ok(())
        }

        #[test]
        fn the_server_peer_has_no_pieces_ok() -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let client_peer =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces()?;

            assert_eq!(
                None,
                client_peer.look_for_a_missing_piece_index(server_piece_index)
            );
            Ok(())
        }

        #[test]
        fn the_server_peer_has_the_hole_file_and_the_client_peer_has_the_first_piece_ok(
        ) -> Result<(), Box<dyn Error>> {
            let server_piece_index = 0;
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            client_peer.data_of_download.pieces_availability[0] =
                PieceStatus::ValidAndAvailablePiece;

            assert_eq!(
                Some(1),
                client_peer.look_for_a_missing_piece_index(server_piece_index)
            );
            Ok(())
        }
    }

    mod test_update_server_peer_piece_status {

        use super::*;

        #[test]
        fn client_peer_update_piece_status_ok() -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let server_piece_index = 1;
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces()?;

            client_peer.update_server_peer_piece_status(
                server_peer_index,
                server_piece_index,
                PieceStatus::ValidAndAvailablePiece,
            )?;

            if let Some(list_of_peers_data_for_communication) =
                client_peer.list_of_peers_data_for_communication
            {
                if let Some(server_peer_data) =
                    list_of_peers_data_for_communication.get(server_peer_index)
                {
                    if let Some(pieces_availability) = &server_peer_data.pieces_availability {
                        assert_eq!(
                            pieces_availability.get(usize::try_from(server_piece_index)?),
                            Some(&PieceStatus::ValidAndAvailablePiece)
                        );
                        return Ok(());
                    }
                }
            }

            Err(Box::new(TestingError::ClientPeerFieldsInvalidAccess(
                "Couldn`t access to client peer fields.".to_string(),
            )))
        }

        #[test]
        fn client_peer_cannot_update_piece_status_with_invalid_index_error(
        ) -> Result<(), Box<dyn Error>> {
            let server_peer_index = 0;
            let server_piece_index = 2;
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_no_valid_pieces()?;

            assert_eq!(
                Err(MsgLogicControlError::UpdatingPieceStatus(
                    "[MsgLogicControlError] Invalid piece index.".to_string(),
                )),
                client_peer.update_server_peer_piece_status(
                    server_peer_index,
                    server_piece_index,
                    PieceStatus::ValidAndAvailablePiece,
                )
            );

            Ok(())
        }
    }

    mod test_store_block {
        use std::fs;

        use super::*;

        #[test]
        fn the_received_block_is_smaller_than_expected_error() -> Result<(), Box<dyn Error>> {
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block = vec![];

            let path = "test_client/store_block_1".to_string();

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] Block length is not as expected".to_string()
                )),
                client_peer.store_block(piece_index, beginning_byte_index, block, &path)
            );

            Ok(())
        }

        #[test]
        fn the_received_block_is_bigger_than_expected_error() -> Result<(), Box<dyn Error>> {
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block = [0; BLOCK_BYTES as usize + 1].to_vec();

            let path = "test_client/store_block_2".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] Block length is not as expected".to_string()
                )),
                client_peer.store_block(piece_index, beginning_byte_index, block, &path)
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn the_received_piece_index_is_invalid_error() -> Result<(), Box<dyn Error>> {
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 2;
            let beginning_byte_index = 0;
            let block = [0; BLOCK_BYTES as usize].to_vec();

            let path = "test_client/store_block_3".to_string();
            fs::create_dir(format!("temp/{}", path))?;

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] The received piece index is invalid.".to_string(),
                )),
                client_peer.store_block(piece_index, beginning_byte_index, block, &path)
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn the_client_peer_receives_one_block_ok() -> Result<(), Box<dyn Error>> {
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block = [0; 16384].to_vec();

            let path = "test_client/store_block_4".to_string();

            client_peer.store_block(piece_index, beginning_byte_index, block, &path)?;

            if let Some(PieceStatus::PartiallyDownloaded { downloaded_bytes }) = client_peer
                .data_of_download
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
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 1;
            let beginning_byte_index = 0;
            let block = [0; 6000].to_vec();

            let path = "test_client/store_block_5".to_string();

            client_peer.store_block(piece_index, beginning_byte_index, block, &path)?;

            if let Some(piece_status) = client_peer
                .data_of_download
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
        fn the_client_peer_receives_a_piece_that_already_own_error() -> Result<(), Box<dyn Error>> {
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 1;
            let mut beginning_byte_index = 0;
            let block = [0; 6000].to_vec();

            let path = "test_client/store_block_6".to_string();

            client_peer.store_block(
                piece_index,
                beginning_byte_index,
                block.clone(),
                &path.clone(),
            )?;
            beginning_byte_index = 6000;

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] The client peer has already completed that piece."
                        .to_string(),
                )),
                client_peer.store_block(piece_index, beginning_byte_index, block, &path)
            );
            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn the_client_peer_receives_two_blocks_ok() -> Result<(), Box<dyn Error>> {
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 0;
            let mut beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();

            let path = "test_client/store_block_7".to_string();

            client_peer.store_block(piece_index, beginning_byte_index, block_1, &path)?;
            beginning_byte_index = 16384;
            client_peer.store_block(piece_index, beginning_byte_index, block_2, &path)?;

            if let Some(PieceStatus::PartiallyDownloaded { downloaded_bytes }) = client_peer
                .data_of_download
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
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 0;
            let mut beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let block_3 = [0; 34000 - (2 * 16384)].to_vec();

            let path = "test_client/store_block_8".to_string();

            client_peer.store_block(piece_index, beginning_byte_index, block_1, &path)?;
            beginning_byte_index = 16384;
            client_peer.store_block(piece_index, beginning_byte_index, block_2, &path)?;
            beginning_byte_index = 16384 * 2;
            client_peer.store_block(piece_index, beginning_byte_index, block_3, &path)?;

            if let Some(piece_status) = client_peer
                .data_of_download
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
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 0;
            let beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();

            let path = "test_client/store_block_9".to_string();

            client_peer.store_block(piece_index, beginning_byte_index, block_1, &path)?;

            assert_eq!(
                Err(MsgLogicControlError::StoringBlock(
                    "[MsgLogicControlError] The beginning byte index is incorrect.".to_string(),
                )),
                client_peer.store_block(piece_index, beginning_byte_index, block_2, &path)
            );

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }

        #[test]
        fn the_client_peer_receives_three_blocks_and_updates_downloaded_data_ok(
        ) -> Result<(), Box<dyn Error>> {
            let mut client_peer =
                create_default_client_peer_with_a_server_peer_that_has_the_hole_file()?;
            let piece_index = 0;
            let mut beginning_byte_index = 0;
            let block_1 = [0; 16384].to_vec();
            let block_2 = [0; 16384].to_vec();
            let block_3 = [0; 34000 - (2 * 16384)].to_vec();

            let path = "test_client/store_block_10".to_string();

            assert_eq!(0, client_peer.data_of_download.downloaded);
            assert_eq!(40000, client_peer.data_of_download.left);

            client_peer.store_block(piece_index, beginning_byte_index, block_1, &path)?;
            assert_eq!(16384, client_peer.data_of_download.downloaded);
            assert_eq!(40000 - 16384, client_peer.data_of_download.left);

            beginning_byte_index = 16384;

            client_peer.store_block(piece_index, beginning_byte_index, block_2, &path)?;
            assert_eq!(16384 * 2, client_peer.data_of_download.downloaded);
            assert_eq!(40000 - 16384 * 2, client_peer.data_of_download.left);

            beginning_byte_index = 16384 * 2;

            client_peer.store_block(piece_index, beginning_byte_index, block_3, &path)?;
            assert_eq!(34000, client_peer.data_of_download.downloaded);
            assert_eq!(40000 - 34000, client_peer.data_of_download.left);

            fs::remove_dir_all(format!("temp/{}", path))?;
            Ok(())
        }
    }
}
