use crate::torrent::data::tracker_response_data::TrackerResponseData;
use std::{error::Error, fmt};

pub struct Client {
    pub tracker_response: TrackerResponseData,
}

impl Client {
    //se podria implementar la creacin de un cliente a partir de un bloqe de datos dado o algo por el estilo
    //fn new() -> Self {}
}

// ---------

#[derive(PartialEq, Debug)]
pub enum ClientError {
    ConectingWithPeerError(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {:?}", self)
    }
}

impl Error for ClientError {}

// -------
