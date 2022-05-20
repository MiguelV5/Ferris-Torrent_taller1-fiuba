#![allow(dead_code)]
use crate::torrent::parsers::p2p::message::*;
use crate::torrent::{client::Client, parsers::p2p};
use std::{io::Write, net::TcpStream};

// otra manera de hacer un socket address:
// SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)

pub const DEFAULT_ADDR: &str = "127.0.0.1:8080";

#[derive(PartialEq, Debug)]
pub enum MsgLogicControlError {
    ConectingWithPeerError(String),
}

fn handle_client_comunication(peer_client: Client) -> Result<(), MsgLogicControlError> {
    //habria que ver si lo que nos pasan es una refernecia o el ownership

    //conexion con un peer
    let peer_data = match peer_client.tracker_response.peers.get(0) {
        Some(peer_data) => peer_data,
        None => return Ok(()), //ver
    };
    let peer_address = &peer_data.peer_address;
    //cambiar el texto dentro del error.
    let mut stream = TcpStream::connect(peer_address)
        .map_err(|error| MsgLogicControlError::ConectingWithPeerError(format!("{:?}", error)))?;

    //envio un handshake
    //esto lo deberia hacer el sender igual
    let handshake_bytes = p2p::encoder::to_bytes(P2PMessage::Handshake {
        protocol_str: "BitTorrent protocol".to_string(),
        info_hash: [0; 20].to_vec(),
        peer_id: "-FA0001-000000000000".to_string(), //los numeros son aleatorios
    })
    .unwrap();
    stream.write_all(&handshake_bytes).unwrap();

    //de ahora en mas me quedo recibiendo mensajes nomas y en base a eso accciono.

    //recibo un handshake

    Ok(())
}

#[cfg(test)]
mod test_msg_logic_control {
    use super::*;
    use crate::torrent::client::data::tracker_response_data::{
        TrackerResponseData, TrackerResponsePeerData,
    };
    use crate::torrent::{client::Client, parsers::p2p};
    use std::io::Read;
    use std::net::{SocketAddr, TcpListener};
    use std::str::FromStr;

    #[test]
    fn test01() {
        // ABRO LA CONEXION
        let listener = TcpListener::bind(DEFAULT_ADDR).unwrap();
        let mut bytes = [0; 68].to_vec(); //ver la cte de la cantidad de bytes de un bytes

        // CREACION DE UN CLIENTE PEER
        let peer = TrackerResponsePeerData {
            peer_id: Some("-FA0001-000000000000".to_string()),
            peer_address: SocketAddr::from_str(DEFAULT_ADDR).unwrap(), //ojo con el unwrap ese
        };
        let tracker_response = TrackerResponseData {
            interval: 0,
            tracker_id: "Tracker ID".to_string(),
            complete: 1,
            incomplete: 0,
            peers: vec![peer],
        };
        let peer_client = Client { tracker_response };

        handle_client_comunication(peer_client).unwrap();

        //RECIBO LO QUE ME DEBERIA HABER MANDADO EL CLIENTE
        let (mut stream, _addr) = listener.accept().unwrap();
        stream.read(&mut bytes).unwrap();
        let received_message = p2p::decoder::from_bytes(&bytes).unwrap();

        assert_eq!(
            P2PMessage::Handshake {
                protocol_str: "BitTorrent protocol".to_string(),
                info_hash: [0; 20].to_vec(),
                peer_id: "-FA0001-000000000000".to_string(),
            },
            received_message
        )
    }
}
