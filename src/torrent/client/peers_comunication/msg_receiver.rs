#![allow(dead_code)]

use crate::torrent::parsers::p2p::{
    self, constants::TOTAL_NUM_OF_BYTES_HANDSHAKE, message::P2PMessage,
};
use core::fmt;
use std::{error::Error, io::Read, net::TcpStream, time::Duration};

#[derive(PartialEq, Debug)]
pub enum MsgReceiverError {
    InternalParsing(String),
    ReceivingHanshake(String),
    ReceivingMessage(String),
    ReceivingLenghtPrefix(String),
}

impl fmt::Display for MsgReceiverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for MsgReceiverError {}

/* FALTA:
 * - Documentar
 * - Ver el tema del tiempo a la hora de leer
 */

///
///
pub fn receive_handshake(stream: &mut TcpStream) -> Result<P2PMessage, MsgReceiverError> {
    stream
        .set_read_timeout(Some(Duration::new(0, 1)))
        .map_err(|err| MsgReceiverError::ReceivingHanshake(format!("{:?}", err)))?; //ver este tiempo porque capaz tiene que ser seteado desde afuera

    let mut buffer = [0; TOTAL_NUM_OF_BYTES_HANDSHAKE].to_vec();
    stream
        .read_exact(&mut buffer)
        .map_err(|error| MsgReceiverError::ReceivingHanshake(format!("{:?}", error)))?;

    let message = p2p::decoder::from_bytes(&buffer)
        .map_err(|err| MsgReceiverError::InternalParsing(format!("{:?}", err)))?;
    Ok(message)
}

///
///
pub fn receive_message(stream: &mut TcpStream) -> Result<P2PMessage, MsgReceiverError> {
    //falta modularizacion en la funcion pero ya funciona todo correctamente
    stream
        .set_read_timeout(Some(Duration::new(0, 1)))
        .map_err(|err| MsgReceiverError::ReceivingMessage(format!("{:?}", err)))?; //ver este tiempo porque capaz tiene que ser seteado desde afuera

    // LEO LENGHT PREFIX
    let mut buffer_lenght_prefix = [0; 4].to_vec();
    stream
        .read_exact(&mut buffer_lenght_prefix)
        .map_err(|error| MsgReceiverError::ReceivingLenghtPrefix(format!("{:?}", error)))?;

    //GENERO EL VALOR DEL LENGHT PREFIX
    let lenght_prefix_value = p2p::decoder::concatenate_bytes_into_u32(&buffer_lenght_prefix)
        .map_err(|err| MsgReceiverError::ReceivingMessage(format!("{:?}", err)))?; //ver si esa funcion realmente debe estar en el parser
    let lenght_prefix_value = u32::try_into(lenght_prefix_value)
        .map_err(|err| MsgReceiverError::ReceivingMessage(format!("{:?}", err)))?; // (Miguel): (Ver primero nota de client_struct.rs) lo cambié de MsgReceiverError::FromU32ToUSizeError a esto. En general como estamos encapsulandolos ya queda adentro la info de qué fue exactamente el error, y asi queda más claro de qué funcion del cliente viene.

    // LEO LOS BYTES RESTANTES SEGUN LENGHT PREFIX
    let mut buffer_msg = Vec::with_capacity(lenght_prefix_value);
    buffer_msg.resize_with(lenght_prefix_value, Default::default);

    stream
        .read_exact(&mut buffer_msg)
        .map_err(|error| MsgReceiverError::ReceivingMessage(format!("{:?}", error)))?;

    let mut bytes = buffer_lenght_prefix;
    bytes.append(&mut buffer_msg);

    let message = p2p::decoder::from_bytes(&bytes)
        .map_err(|err| MsgReceiverError::ReceivingMessage(format!("{:?}", err)))?;
    Ok(message)
}

#[cfg(test)]
mod test_msg_receiver {
    use super::*;
    use crate::torrent::parsers::p2p::constants::PSTR_STRING_HANDSHAKE;
    use std::error::Error;
    use std::{io::Write, net::TcpListener};

    //
    // AUX PARA CONEXIONES:
    use std::io::ErrorKind;
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

    //
    //

    mod test_receive_handshake {
        use super::*;

        #[test]
        fn receive_handshake_ok() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let mut sender_stream = TcpStream::connect(address)?;
            let (mut receptor_stream, _addr) = listener.accept()?;

            let handshake = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: [1; 20].to_vec(),
                peer_id: "-FA0001-000000000000".bytes().collect(),
            };

            let buffer = p2p::encoder::to_bytes(handshake.clone())?;
            sender_stream.write(&buffer)?;

            assert_eq!(handshake, receive_handshake(&mut receptor_stream)?);
            Ok(())
        }

        #[test]
        fn receive_hanshake_with_less_bytes_error() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let mut sender_stream = TcpStream::connect(address)?;
            let (mut receptor_stream, _addr) = listener.accept()?;

            let handshake = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: [1; 20].to_vec(),
                peer_id: "-FA0001-000000000000".bytes().collect(),
            };

            let mut buffer = p2p::encoder::to_bytes(handshake.clone())?;
            buffer.pop();
            sender_stream.write(&buffer)?;

            assert!(receive_handshake(&mut receptor_stream).is_err());
            Ok(())
        }

        #[test]
        fn receive_hanshake_with_invalid_fields_error() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let mut sender_stream = TcpStream::connect(address)?;
            let (mut receptor_stream, _addr) = listener.accept()?;

            let handshake = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                info_hash: [1; 20].to_vec(),
                peer_id: "-FA0001-000000000000".bytes().collect(),
            };

            let mut buffer = p2p::encoder::to_bytes(handshake.clone())?;
            buffer[0] = 0; //cambio el campo pstrlen del handshake por cero.
            sender_stream.write(&buffer)?;

            assert!(receive_handshake(&mut receptor_stream).is_err());
            Ok(())
        }
    }

    mod test_receive_message {
        use super::*;

        #[test]
        fn receive_message_keep_alive_ok() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let mut sender_stream = TcpStream::connect(address)?;
            let (mut receptor_stream, _addr) = listener.accept()?;

            let message = P2PMessage::KeepAlive;

            let buffer = p2p::encoder::to_bytes(message.clone())?;
            sender_stream.write(&buffer)?;

            assert_eq!(message, receive_message(&mut receptor_stream)?);

            Ok(())
        }

        #[test]
        fn receive_message_with_id_ok() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let mut sender_stream = TcpStream::connect(address)?;
            let (mut receptor_stream, _addr) = listener.accept()?;

            let message = P2PMessage::Choke;

            let buffer = p2p::encoder::to_bytes(message.clone())?;
            sender_stream.write(&buffer)?;

            assert_eq!(message, receive_message(&mut receptor_stream)?);

            Ok(())
        }

        #[test]
        fn receive_message_with_id_and_payload_ok() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let mut sender_stream = TcpStream::connect(address)?;
            let (mut receptor_stream, _addr) = listener.accept()?;

            let message = P2PMessage::Have { piece_index: 1 };

            let buffer = p2p::encoder::to_bytes(message.clone())?;
            sender_stream.write(&buffer)?;

            assert_eq!(message, receive_message(&mut receptor_stream)?);

            Ok(())
        }

        #[test]
        fn receive_message_with_more_than_one_msg_ok() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let mut sender_stream = TcpStream::connect(address)?;
            let (mut receptor_stream, _addr) = listener.accept()?;

            let message1 = P2PMessage::Choke;
            let message2 = P2PMessage::Unchoke;
            let message3 = P2PMessage::Have { piece_index: 1 };

            let buffer = p2p::encoder::to_bytes(message1.clone())?;
            sender_stream.write(&buffer)?;
            let buffer = p2p::encoder::to_bytes(message2.clone())?;
            sender_stream.write(&buffer)?;
            let buffer = p2p::encoder::to_bytes(message3.clone())?;
            sender_stream.write(&buffer)?;

            assert_eq!(message1, receive_message(&mut receptor_stream)?);
            assert_eq!(message2, receive_message(&mut receptor_stream)?);
            assert_eq!(message3, receive_message(&mut receptor_stream)?);

            Ok(())
        }

        #[test]
        fn receive_message_with_less_bytes_error() -> Result<(), Box<dyn Error>> {
            let (listener, address) = try_bind_listener(STARTING_PORT)?;
            let mut sender_stream = TcpStream::connect(address)?;
            let (mut receptor_stream, _addr) = listener.accept()?;

            let message = P2PMessage::Have { piece_index: 1 };

            let mut buffer = p2p::encoder::to_bytes(message.clone())?;
            buffer.pop();
            sender_stream.write(&buffer)?;

            assert!(receive_message(&mut receptor_stream).is_err());

            Ok(())
        }
    }
}
