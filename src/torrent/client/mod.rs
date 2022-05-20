pub mod peers_comunication;

use crate::torrent::data::tracker_response_data::TrackerResponseData;
pub struct Client {
    tracker_response: TrackerResponseData,
}

impl Client {
    //se podria implementar la creacin de un cliente a partir de un bloqe de datos dado o algo por el estilo
    //fn new() -> Self {}
}
