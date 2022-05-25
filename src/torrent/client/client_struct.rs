use crate::torrent::data::{
    client_data::ClientData, peers_data::PeersDataList, torrent_file_data::TorrentFileData,
    tracker_response_data::TrackerResponseData,
};
use std::{error::Error, fmt};

#[derive(PartialEq, Debug, Clone)]
pub struct Client {
    pub client_data: ClientData, //cambiar nombre despues. Tambien ver porque podria ser la misma estrucura que PeerData
    pub torrent_file: TorrentFileData,
    pub tracker_response: TrackerResponseData, //deberia ser un Option<>
    pub peers_data_list: Option<PeersDataList>,
}

// impl Client {
//     se podria implementar la creacin de un cliente a partir de un bloqe de datos dado o algo por el estilo
//     fn new() -> Self {}
// }

#[derive(PartialEq, Debug)]
pub enum ClientError {
    ConectingWithPeer(String),
    ReceivingHanshake(String),
    ReceivingLenghtPrefix(String),
    ReceivingMessage(String),
    SendingMessage(String),
    SendingHanshake(String),
    InternalParsing(String),
    UpdatingBitfield(String),
    CheckingAndSavingHandshake(String),
    //FromU32ToUSizeError(String), // Esto lo quité por: Nota en linea 44 del msg_receiver.rs;
    // NOTA GENERAL ERRORES (Miguel): Estuve pensando y no me parece malo combinar lo que teniamos planeado con la "acumulacion" de errores desde abajo. Es decir, para poder ver claramente TODO el backtrace del error, sabiendo asi exactamente cuando, donde y por qué funciones fue que se fue propagando el error. Haciendo eso combinandolo con lo del String creo que no queda mal. Además de esta forma podemos evitarnos el monton de verbose que llevan nuestras funciones al usar siempre Boxes. Lo que si está bueno (que no quité) es dejar esa forma de burbujear SOLO para los tests, ya que ahi no nos importa tanto ver todo el backtrace y solo queremos ver si fallo por el assert o por otra cosa que nada que ver.
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error del Cliente.\n Backtrace: {:?}\n", self)
    }
}

impl Error for ClientError {}
