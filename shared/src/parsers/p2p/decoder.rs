//! # Modulo de decodificacion P2P
//! Este modulo contiene las funciones encargadas de decodificar mensajes P2P provenientes de sockets, los cuales llegan en bytes
//! y deben ser interpretados para realizar la logica necesaria de recepcion de mensajes.

use super::constants::*;
use super::message::*;

///
/// A partir de 4 bytes (u8) recibidos en un slice, devuelve la "concatenacion" de dichos bytes para formar un u32 completo.
/// Tener en cuenta que el slice de bytes esperado debe estar ordenado a modo big endian.
///
/// Devuelve un Result donde la variante Ok es el u32, y la variante Err es un P2PMessageDecodingError
///
///
pub fn concatenate_bytes_into_u32(bytes: &[u8]) -> Result<u32, P2PMessageDecodingError> {
    if bytes.len() != NEEDED_NUM_OF_BYTES_TO_CONCATENATE {
        return Err(P2PMessageDecodingError::ByteAmount(
            "[P2PMessageDecodingError] Invalid amount of bytes to concatenate into an u32 (4 are required)".to_string()
        ));
    }

    let mut concatenation = u32::from(bytes[0]);
    for byte in bytes.iter() {
        concatenation <<= NUM_BITS_ON_A_BYTE;
        concatenation += u32::from(*byte);
    }
    Ok(concatenation)
}

///
/// A partir de una cadena de bytes (u8) recibidos en un slice, se verifica si dicha cadena posee el formato de un handshake devolviendo true en dicho caso o false en caso contrario.
///
/// Las verificaciones que se realizan son respecto a la cantidad esperada de bytes que contiene un handshake y al campo pstr, el cual debe poseer el formato "BitTorrent protocol". (Segun especificacion).
///
/// Tener en cuenta que el slice de bytes esperado debe estar ordenado a modo big endian.
///
fn is_handshake(bytes: &[u8]) -> bool {
    if bytes.len() != TOTAL_NUM_OF_BYTES_HANDSHAKE {
        return false;
    }

    let pstrlen = bytes[0];
    if pstrlen != PSTRLEN_VALUE_HANDSHAKE {
        return false;
    }

    if let Ok(pstr) = String::from_utf8(bytes[1..20].to_vec()) {
        return pstr == PSTR_STRING_HANDSHAKE;
    }

    false
}

///
/// A partir de una cadena de bytes (u8) recibidos en un slice con los datos validos de un handshake, intenta generar un mensaje p2p del tipo "handshake".
///
/// Cabe destacar que es necesario que los parametros recibidos deben ser siempre validos, tanto en relacion a la informacion como tambien al tamaño.
///
/// Tener en cuenta que el slice de bytes esperado debe estar ordenado a modo big endian.
///
fn try_decode_handshake_p2p_message(bytes: &[u8]) -> Result<P2PMessage, P2PMessageDecodingError> {
    let protocol_str = String::from_utf8(bytes[1..20].to_vec()).map_err(|err| {
        P2PMessageDecodingError::FromBytesToString(format!("[P2PMessageDecodingError] {:?}", err))
    })?;
    let info_hash = bytes[28..48].to_vec();
    let peer_id = bytes[48..68].to_vec();
    Ok(P2PMessage::Handshake {
        protocol_str,
        info_hash,
        peer_id,
    })
}

///
/// A partir de una cadena de bytes (u8) recibidos en un slice y un entero de 32 bits no signado que representa el lenght_prefix de un mensaje p2p, devuelve un tipo de dato Result<book, P2PMessageDecodingError>.
/// Este booleno devuelto significa:
/// true   -> si la cantidad de bytes que hay en el slice para generar un mensaje p2p es la correcta.
/// falsea -> en caso contrario
///
fn has_the_correct_number_of_bytes(
    bytes: &[u8],
    lenght_prefix: u32,
) -> Result<bool, P2PMessageDecodingError> {
    match u32::try_from(bytes.len() - NUM_OF_BYTES_LENGHT_PREFIX) {
        Ok(bytes_lenght) => Ok(bytes_lenght == lenght_prefix),
        Err(err) => Err(P2PMessageDecodingError::FromUsizeToU32(format!(
            "[P2PMessageDecodingError] {:?}",
            err
        ))),
    }
}

///
/// A partir de una cadena de bytes (u8) recibidos en un slice con los datos validos de un bitfield, genera un mensaje p2p del tipo "bitfield".
///
/// Cabe destacar que es necesario que los parametros recibidos deben ser siempre validos, tanto en relacion a la informacion como tambien al tamaño.
///
/// Tener en cuenta que el slice de bytes esperado debe estar ordenado a modo big endian.
///
fn decode_bitfield_p2p_message(bytes: &[u8]) -> P2PMessage {
    let mut bitfield: Vec<PieceStatus> = Vec::with_capacity(bytes.len() * NUM_BITS_ON_A_BYTE);

    for (i, value) in bytes.iter().enumerate() {
        let mut byte = *value as i8;
        for j in 0..NUM_BITS_ON_A_BYTE {
            if byte.is_negative() {
                bitfield.insert(
                    i * NUM_BITS_ON_A_BYTE + j,
                    PieceStatus::ValidAndAvailablePiece,
                )
            } else {
                bitfield.insert(
                    i * NUM_BITS_ON_A_BYTE + j,
                    PieceStatus::MissingPiece {
                        was_requested: false,
                    },
                )
            }
            byte <<= 1;
        }
    }

    P2PMessage::Bitfield { bitfield }
}

///
/// A partir de una cadena de bytes (u8) recibidos en un slice con los datos validos de un have, intenta generar un mensaje p2p del tipo "have".
///
/// Cabe destacar que es necesario que los parametros recibidos deben ser siempre validos, tanto en relacion a la informacion como tambien al tamaño.
///
/// Tener en cuenta que el slice de bytes esperado debe estar ordenado a modo big endian.
///
fn try_decode_have_p2p_message(bytes: &[u8]) -> Result<P2PMessage, P2PMessageDecodingError> {
    let piece_index = concatenate_bytes_into_u32(&bytes[0..4])?;
    Ok(P2PMessage::Have { piece_index })
}

///
/// A partir de una cadena de bytes (u8) recibidos en un slice con los datos validos de un request, intenta generar un mensaje p2p del tipo "request".
///
/// Cabe destacar que es necesario que los parametros recibidos deben ser siempre validos, tanto en relacion a la informacion como tambien al tamaño.
///
/// Tener en cuenta que el slice de bytes esperado debe estar ordenado a modo big endian.
///
fn try_decode_request_p2p_message(bytes: &[u8]) -> Result<P2PMessage, P2PMessageDecodingError> {
    let piece_index = concatenate_bytes_into_u32(&bytes[0..4])?;
    let beginning_byte_index = concatenate_bytes_into_u32(&bytes[4..8])?;
    let amount_of_bytes = concatenate_bytes_into_u32(&bytes[8..12])?;
    Ok(P2PMessage::Request {
        piece_index,
        beginning_byte_index,
        amount_of_bytes,
    })
}

///
/// A partir de una cadena de bytes (u8) recibidos en un slice con los datos validos de un piece, intenta generar un mensaje p2p del tipo "piece".
///
/// Cabe destacar que es necesario que los parametros recibidos deben ser siempre validos, tanto en relacion a la informacion como tambien al tamaño.
///
/// Tener en cuenta que el slice de bytes esperado debe estar ordenado a modo big endian.
///
fn try_decode_piece_p2p_message(bytes: &[u8]) -> Result<P2PMessage, P2PMessageDecodingError> {
    let piece_index = concatenate_bytes_into_u32(&bytes[0..4])?;
    let beginning_byte_index = concatenate_bytes_into_u32(&bytes[4..8])?;
    let block = bytes[8..].to_vec();
    Ok(P2PMessage::Piece {
        piece_index,
        beginning_byte_index,
        block,
    })
}

///
/// A partir de una cadena de bytes (u8) recibidos en un slice con los datos validos de un cancel, intenta generar un mensaje p2p del tipo "cancel".
///
/// Cabe destacar que es necesario que los parametros recibidos deben ser siempre validos, tanto en relacion a la informacion como tambien al tamaño.
///
/// Tener en cuenta que el slice de bytes esperado debe estar ordenado a modo big endian.
///
fn try_decode_cancel_p2p_message(bytes: &[u8]) -> Result<P2PMessage, P2PMessageDecodingError> {
    let piece_index = concatenate_bytes_into_u32(&bytes[0..4])?;
    let beginning_byte_index = concatenate_bytes_into_u32(&bytes[4..8])?;
    let amount_of_bytes = concatenate_bytes_into_u32(&bytes[8..12])?;
    Ok(P2PMessage::Cancel {
        piece_index,
        beginning_byte_index,
        amount_of_bytes,
    })
}

///
/// A partir de una cadena de bytes (u8) recibidos en un slice con los datos validos de un port, intenta generar un mensaje p2p del tipo "port".
///
/// Cabe destacar que es necesario que los parametros recibidos deben ser siempre validos, tanto en relacion a la informacion como tambien al tamaño.
///
/// Tener en cuenta que el slice de bytes esperado debe estar ordenado a modo big endian.
///
fn try_decode_port_p2p_message(bytes: &[u8]) -> Result<P2PMessage, P2PMessageDecodingError> {
    let port_bytes = [0u8, 0u8, bytes[0], bytes[1]];
    let port_value = concatenate_bytes_into_u32(&port_bytes)?;
    Ok(P2PMessage::Port {
        listen_port: port_value,
    })
}

// Matchea la id del mensaje p2p con su representacion correspondiente.
// Devuelve un Result tal que:
// - El Ok value es una variante de P2PMessage segun sea adecuado.
// - El Err value es una variante de P2PMessageDecodingError si no se pudo interpretar el mensaje.
fn match_p2p_msg_according_to_id(bytes: &[u8]) -> Result<P2PMessage, P2PMessageDecodingError> {
    let id_byte = bytes[4];

    match id_byte {
        ID_CHOKE => Ok(P2PMessage::Choke),
        ID_UNCHOKE => Ok(P2PMessage::Unchoke),
        ID_INTERESTED => Ok(P2PMessage::Interested),
        ID_NOT_INTERESTED => Ok(P2PMessage::NotInterested),
        ID_HAVE => try_decode_have_p2p_message(&bytes[5..]),
        ID_BITFIELD => Ok(decode_bitfield_p2p_message(&bytes[5..])),
        ID_REQUEST => try_decode_request_p2p_message(&bytes[5..]),
        ID_PIECE => try_decode_piece_p2p_message(&bytes[5..]),
        ID_CANCEL => try_decode_cancel_p2p_message(&bytes[5..]),
        ID_PORT => try_decode_port_p2p_message(&bytes[5..]),
        _ => Err(P2PMessageDecodingError::InvalidId(
            "[P2PMessageDecodingError] Tried to decode a message with invalid ID".to_string(),
        )),
    }
}

// Determina cual es tipo de mensaje p2p en base a los bytes recibidos y al lenght_prefix. El lenght_prefix almacena la informacion sobre la cantidad de bytes que deben ser leidos para formar el mensaje correctamente.
//
fn determinate_p2p_msg(
    bytes: &[u8],
    lenght_prefix: u32,
) -> Result<P2PMessage, P2PMessageDecodingError> {
    if lenght_prefix == 0 {
        Ok(P2PMessage::KeepAlive)
    } else {
        match_p2p_msg_according_to_id(bytes)
    }
}

/// Recibe un slice con todos los bytes correspondientes a un mensaje P2P a interpretar.
/// Devuelve un Result tal que:
/// - El Ok value es una variante de P2PMessage segun sea adecuado tras interpretar los bytes.
/// - El Err value es una variante de P2PMessageDecodingError si no se pudo interpretar el mensaje.
///
// # Ejemplo de uso básico:
//
// ```
// # use shared::parsers::p2p;
// # use shared::parsers::p2p::message::*;
// let p2p_msg_bytes = [0, 0, 0, 13, 6, 0, 0, 0, 10, 0, 0, 0, 5, 0, 0, 0, 3];
// assert_eq!(
//     Ok(P2PMessage::Request {
//         piece_index: 10,
//         beginning_byte_index: 5,
//         amount_of_bytes: 3
//     }),
//     p2p::decoder::from_bytes(&p2p_msg_bytes)
// );
// ```
///
pub fn from_bytes(bytes: &[u8]) -> Result<P2PMessage, P2PMessageDecodingError> {
    if bytes.len() < MIN_BYTES_OF_A_P2P_MSG {
        return Err(P2PMessageDecodingError::ByteAmount(
            "[P2PMessageDecodingError] The P2P msg to decode does not have enough bytes (min. 4 required)".to_string()
        ));
    }

    if is_handshake(bytes) {
        return try_decode_handshake_p2p_message(bytes);
    }

    let lenght_prefix = concatenate_bytes_into_u32(&bytes[0..4])?;
    if !has_the_correct_number_of_bytes(bytes, lenght_prefix)? {
        Err(P2PMessageDecodingError::ByteAmount(
            "[P2PMessageDecodingError] The true length of the P2P msg does not match the one given in the length prefix".to_string()
        ))
    } else {
        determinate_p2p_msg(bytes, lenght_prefix)
    }
}

#[cfg(test)]
mod tests_p2p_decoder {
    use super::*;

    mod tests_concatenate_bytes_into_u32 {
        use super::*;

        #[test]
        fn concatenate_bytes_into_u32_from_less_than_four_bytes_error() {
            let bytes = [0];
            assert_eq!(
                Err(P2PMessageDecodingError::ByteAmount(format!(
                    "[P2PMessageDecodingError] Invalid amount of bytes to concatenate into an u32 (4 are required)"
                ))),
                concatenate_bytes_into_u32(&bytes)
            );
        }

        #[test]
        fn concatenate_bytes_into_u32_from_more_than_four_bytes_error() {
            let bytes = [0, 0, 0, 1, 2];
            assert_eq!(
                Err(P2PMessageDecodingError::ByteAmount(format!(
                    "[P2PMessageDecodingError] Invalid amount of bytes to concatenate into an u32 (4 are required)"
                ))),
                concatenate_bytes_into_u32(&bytes)
            );
        }

        #[test]
        fn concatenate_bytes_into_u32_from_four_zeroed_bytes_ok() {
            let bytes = [0; 4];
            let expected_value = 0;
            assert_eq!(Ok(expected_value), concatenate_bytes_into_u32(&bytes));
        }

        #[test]
        fn concatenate_bytes_into_u32_from_four_bytes_ok() {
            let bytes = [1, 1, 1, 1];
            let expected_value = 16843009;
            assert_eq!(Ok(expected_value), concatenate_bytes_into_u32(&bytes));
        }
    }

    mod tests_is_handshake {
        use super::*;

        #[test]
        fn is_handshake_with_less_than_sixty_eight_bytes_error() {
            let p2p_msg_bytes = [0];
            assert!(!is_handshake(&p2p_msg_bytes));
        }

        #[test]
        fn is_handshake_with_more_than_sixty_eight_bytes_error() {
            let p2p_msg_bytes = [0; 70];
            assert!(!is_handshake(&p2p_msg_bytes));
        }

        #[test]
        fn is_handshake_with_an_invalid_pstrlen_error() {
            let mut p2p_msg_bytes = [0; TOTAL_NUM_OF_BYTES_HANDSHAKE];
            p2p_msg_bytes[0] = 20;
            assert!(!is_handshake(&p2p_msg_bytes));
        }

        #[test]
        fn is_handshake_with_an_invalid_pstr_error() {
            let pstr_bytes = String::from("VitTorrent protocol").as_bytes().to_vec();
            let mut p2p_msg_bytes = [0; TOTAL_NUM_OF_BYTES_HANDSHAKE];
            p2p_msg_bytes[0] = PSTRLEN_VALUE_HANDSHAKE;
            for i in 0..19 {
                p2p_msg_bytes[i + 1] = pstr_bytes[i];
            }
            assert!(!is_handshake(&p2p_msg_bytes));
        }

        #[test]
        fn is_handshake_with_the_correct_fields_ok() {
            let pstr_bytes = String::from(PSTR_STRING_HANDSHAKE).as_bytes().to_vec();
            let mut p2p_msg_bytes = [0; TOTAL_NUM_OF_BYTES_HANDSHAKE];
            p2p_msg_bytes[0] = PSTRLEN_VALUE_HANDSHAKE;
            for i in 0..19 {
                p2p_msg_bytes[i + 1] = pstr_bytes[i];
            }
            assert!(is_handshake(&p2p_msg_bytes));
        }
    }

    mod tests_decode_bitfield_p2p_message {
        use super::*;

        #[test]
        fn decode_bitfield_from_one_byte() {
            let bifield_bytes = [1];
            assert_eq!(
                P2PMessage::Bitfield {
                    bitfield: vec![
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::ValidAndAvailablePiece
                    ]
                },
                decode_bitfield_p2p_message(&bifield_bytes)
            );
        }

        #[test]
        fn decode_bitfield_from_some_bytes() {
            let bifield_bytes = [1, 2, 3];
            assert_eq!(
                P2PMessage::Bitfield {
                    bitfield: vec![
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::ValidAndAvailablePiece,
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::ValidAndAvailablePiece,
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::ValidAndAvailablePiece,
                        PieceStatus::ValidAndAvailablePiece
                    ]
                },
                decode_bitfield_p2p_message(&bifield_bytes)
            );
        }
    }

    mod tests_decode_p2p_message {
        use super::*;

        #[test]
        fn decode_less_than_four_bytes_error() {
            let p2p_msg_bytes = [0];
            assert_eq!(
                Err(P2PMessageDecodingError::ByteAmount(format!(
                    "[P2PMessageDecodingError] The P2P msg to decode does not have enough bytes (min. 4 required)"
                ))),
                from_bytes(&p2p_msg_bytes)
            );
        }

        #[test]
        fn decode_handshake() {
            let mut p2p_msg_bytes = [0; TOTAL_NUM_OF_BYTES_HANDSHAKE];
            let pstr_bytes = String::from(PSTR_STRING_HANDSHAKE).as_bytes().to_vec();
            let info_hash_bytes = [1; 20];
            let peer_id_bytes = "-FA0001-012345678901".to_string().as_bytes().to_vec();

            p2p_msg_bytes[0] = PSTRLEN_VALUE_HANDSHAKE;
            for i in 1..20 {
                p2p_msg_bytes[i] = pstr_bytes[i - 1];
            }
            for i in 28..48 {
                p2p_msg_bytes[i] = info_hash_bytes[i - 28];
            }
            for i in 48..68 {
                p2p_msg_bytes[i] = peer_id_bytes[i - 48];
            }

            assert_eq!(
                Ok(P2PMessage::Handshake {
                    protocol_str: PSTR_STRING_HANDSHAKE.to_string(),
                    info_hash: [1; 20].to_vec(),
                    peer_id: "-FA0001-012345678901".bytes().collect(),
                }),
                from_bytes(&p2p_msg_bytes)
            );
        }

        #[test]
        fn decode_keep_alive_ok() {
            let p2p_msg_bytes = [0; 4];
            assert_eq!(Ok(P2PMessage::KeepAlive), from_bytes(&p2p_msg_bytes));
        }

        #[test]
        fn decode_choke_ok() {
            let p2p_msg_bytes = [0, 0, 0, 1, ID_CHOKE];
            assert_eq!(Ok(P2PMessage::Choke), from_bytes(&p2p_msg_bytes));
        }

        #[test]
        fn decode_unchoke_ok() {
            let p2p_msg_bytes = [0, 0, 0, 1, ID_UNCHOKE];
            assert_eq!(Ok(P2PMessage::Unchoke), from_bytes(&p2p_msg_bytes));
        }

        #[test]
        fn decode_interested_ok() {
            let p2p_msg_bytes = [0, 0, 0, 1, ID_INTERESTED];
            assert_eq!(Ok(P2PMessage::Interested), from_bytes(&p2p_msg_bytes));
        }

        #[test]
        fn decode_not_interested_ok() {
            let p2p_msg_bytes = [0, 0, 0, 1, ID_NOT_INTERESTED];
            assert_eq!(Ok(P2PMessage::NotInterested), from_bytes(&p2p_msg_bytes));
        }

        #[test]
        fn decode_have_with_a_piece_index_ok() {
            let p2p_msg_bytes = [0, 0, 0, 5, ID_HAVE, 0, 0, 0, 1];
            assert_eq!(
                Ok(P2PMessage::Have { piece_index: 1 }),
                from_bytes(&p2p_msg_bytes)
            );
        }

        #[test]
        fn decode_bitfield_ok() {
            let bytes = [0, 0, 0, 2, ID_BITFIELD, 3];
            assert_eq!(
                Ok(P2PMessage::Bitfield {
                    bitfield: vec![
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::MissingPiece {
                            was_requested: false
                        },
                        PieceStatus::ValidAndAvailablePiece,
                        PieceStatus::ValidAndAvailablePiece,
                    ]
                },),
                from_bytes(&bytes)
            );
        }

        #[test]
        fn decode_request_ok() {
            let p2p_msg_bytes = [0, 0, 0, 13, ID_REQUEST, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3];
            assert_eq!(
                Ok(P2PMessage::Request {
                    piece_index: 1,
                    beginning_byte_index: 2,
                    amount_of_bytes: 3
                }),
                from_bytes(&p2p_msg_bytes)
            );
        }

        #[test]
        fn decode_piece_ok() {
            let p2p_msg_bytes = [0, 0, 0, 13, ID_PIECE, 0, 0, 0, 1, 0, 0, 0, 2, 0, 1, 2, 3];
            assert_eq!(
                Ok(P2PMessage::Piece {
                    piece_index: 1,
                    beginning_byte_index: 2,
                    block: vec![0, 1, 2, 3]
                }),
                from_bytes(&p2p_msg_bytes)
            );
        }

        #[test]
        fn decode_cancel_ok() {
            let p2p_msg_bytes = [0, 0, 0, 13, ID_CANCEL, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3];
            assert_eq!(
                Ok(P2PMessage::Cancel {
                    piece_index: 1,
                    beginning_byte_index: 2,
                    amount_of_bytes: 3
                }),
                from_bytes(&p2p_msg_bytes)
            );
        }

        #[test]
        fn decode_port_ok() {
            let p2p_msg_bytes = [0, 0, 0, 3, ID_PORT, 0, 10];
            assert_eq!(
                Ok(P2PMessage::Port { listen_port: 10 }),
                from_bytes(&p2p_msg_bytes)
            );
        }
    }
}
