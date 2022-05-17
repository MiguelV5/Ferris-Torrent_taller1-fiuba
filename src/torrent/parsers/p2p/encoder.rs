use std::vec;

use super::{constants::*, message::*};

///
/// Devuelve un vec de bytes de tipo:
/// <len=0>; tal que mide:
/// <4bytes>
fn encode_keep_alive() -> Vec<u8> {
    vec![0; 4]
}

///
/// Devuelve un vec de bytes de tipo:
/// <len=1><id=0>; tal que cada uno mide:
/// <4bytes><1byte>
fn encode_choke() -> Vec<u8> {
    vec![0, 0, 0, 1, ID_CHOKE]
}

///
/// Devuelve un vec de bytes de tipo:
/// <len=1><id=1>; tal que cada uno mide:
/// <4bytes><1byte>
fn encode_unchoke() -> Vec<u8> {
    vec![0, 0, 0, 1, ID_UNCHOKE]
}

///
/// Devuelve un vec de bytes de tipo:
/// <len=1><id=2>; tal que cada uno mide:
/// <4bytes><1byte>
fn encode_interested() -> Vec<u8> {
    vec![0, 0, 0, 1, ID_INTERESTED]
}

///
/// Devuelve un vec de bytes de tipo:
/// <len=1><id=3>; tal que cada uno mide:
/// <4bytes><1byte>
fn encode_not_interested() -> Vec<u8> {
    vec![0, 0, 0, 1, ID_NOT_INTERESTED]
}

///
/// Devuelve un vec de bytes de tipo:
/// <len=5><id=4><piece index>; tal que cada uno mide:
/// <4bytes><1byte><4bytes>
fn encode_have(piece_index: u32) -> Vec<u8> {
    let mut encoded_have: Vec<u8> = vec![0, 0, 0, 5, ID_HAVE];
    piece_index
        .to_be_bytes()
        .iter()
        .for_each(|byte| encoded_have.push(*byte));
    encoded_have
}

///
/// La longitud del payload para el mensaje Bitfield tiene que ser una cantidad de bytes, mientras que los statuses que se reciben representan bits a encodear, por lo cual dichos Statuses pueden no necesariamente completar una cantidad exacta de bytes.
/// Por lo tanto si la cantidad de piezas no es multiplo de 8 se completan los bits sobrantes con ceros, por ende aca se calcula el tamaño adecuado.
fn calc_adequate_ammount_of_bitfield_bytes(ammount_of_bits: usize) -> usize {
    match (ammount_of_bits % 8) == 0 {
        true => ammount_of_bits / 8,
        false => {
            let mut needed_ammount_of_bits = ammount_of_bits;
            while needed_ammount_of_bits % 8 != 0 {
                needed_ammount_of_bits += 1;
            }
            needed_ammount_of_bits / 8
        }
    }
}

fn append_len_prefix_and_id_to_encoded_bitfield(
    payload_length_in_bytes: u32,
    encoded_bitfield: &mut Vec<u8>,
) {
    let length_prefix = NEEDED_NUM_OF_BYTES_FOR_ID + payload_length_in_bytes;

    length_prefix
        .to_be_bytes()
        .iter()
        .for_each(|byte| encoded_bitfield.push(*byte));
    encoded_bitfield.push(ID_BITFIELD);
}

///
/// Calcula y agrega el payload del mensaje Bitfield al vector de bytes del resultado.
/// El calculo se hace viendo cada PieceStatus (que sería cada bit representando si la pieza de su correspondiente indice está
/// disponible o no). Por esto se va acumulando de a bits y cuando se tiene un byte de info completa, se pushea al resultado.
/// Si se acaban los Statuses, se añaden ceros hasta completar ese último byte.
fn append_payload_to_encoded_bitfield(
    payload_length_in_bytes: u32,
    bitfield_piece_statuses: &[PieceStatus],
    encoded_bitfield: &mut Vec<u8>,
) {
    let mut bit_accumulator: u8 = 0;
    let mask_to_set_last_bit: u8 = 0b00000001;
    let mut i = 0;
    let mut bitfield_statuses_iter = bitfield_piece_statuses.iter();
    while i < payload_length_in_bytes * 8 {
        if let Some(piece_status) = bitfield_statuses_iter.next() {
            match *piece_status {
                PieceStatus::ValidAndAvailablePiece => {
                    bit_accumulator <<= 1;
                    bit_accumulator |= mask_to_set_last_bit;
                }
                PieceStatus::MissingPiece => {
                    bit_accumulator <<= 1;
                }
            };
        } else {
            bit_accumulator <<= 1;
        };

        if (i + 1) % 8 == 0 {
            // Cada iteracion+1 multiplo de 8 es cada fin de un byte (+1 pq se indexa desde cero)
            encoded_bitfield.push(bit_accumulator);
            bit_accumulator = 0;
        };

        i += 1;
    }
}

///
/// Si no hubo fallas de conversión, el Ok value es un vec de bytes de tipo:
/// <len=0001+X><id=5><bitfield>; tal que cada uno mide:
/// <4bytes><1byte><Xbytes>
fn encode_bitfield(bitfield_piece_statuses: Vec<PieceStatus>) -> Result<Vec<u8>, P2PMessageError> {
    let ammount_of_bitfield_statuses = bitfield_piece_statuses.len();
    let unconverted_payload_length =
        calc_adequate_ammount_of_bitfield_bytes(ammount_of_bitfield_statuses);

    let converted_payload_length_in_bytes =
        if let Ok(converted_payload_length) = u32::try_from(unconverted_payload_length) {
            converted_payload_length
        } else {
            return Err(P2PMessageError::FromUsizeToU32Error);
        };

    let encoded_bitfield_capacity = NUM_OF_BYTES_LENGHT_PREFIX
        + (NEEDED_NUM_OF_BYTES_FOR_ID as usize)
        + unconverted_payload_length; // Como es una cte no va a pasar que falle por el casteo.

    let mut encoded_bitfield = Vec::with_capacity(encoded_bitfield_capacity);

    append_len_prefix_and_id_to_encoded_bitfield(
        converted_payload_length_in_bytes,
        &mut encoded_bitfield,
    );

    append_payload_to_encoded_bitfield(
        converted_payload_length_in_bytes,
        &bitfield_piece_statuses,
        &mut encoded_bitfield,
    );

    Ok(encoded_bitfield)
}

///
/// Logica de encodificacion común para ambos mensajes.
fn common_encode_for_request_and_cancel(
    piece_index: u32,
    beginning_byte_index: u32,
    amount_of_bytes: u32,
    id: u8,
) -> Vec<u8> {
    let mut encoded_request = vec![0, 0, 0, 13, id];

    let mut piece_index_in_bytes = piece_index.to_be_bytes().to_vec();
    let mut byte_index_in_bytes = beginning_byte_index.to_be_bytes().to_vec();
    let mut amount_of_bytes_in_bytes = amount_of_bytes.to_be_bytes().to_vec();

    encoded_request.append(&mut piece_index_in_bytes);
    encoded_request.append(&mut byte_index_in_bytes);
    encoded_request.append(&mut amount_of_bytes_in_bytes);

    encoded_request
}

///
/// Devuelve un vec de bytes de tipo:
/// <len=13><id=6><index><begin><length>; tal que cada uno mide:
/// <4bytes><1byte><4bytes><4bytes><4bytes>
fn encode_request(piece_index: u32, beginning_byte_index: u32, amount_of_bytes: u32) -> Vec<u8> {
    common_encode_for_request_and_cancel(
        piece_index,
        beginning_byte_index,
        amount_of_bytes,
        ID_REQUEST,
    )
}

///
/// Si no hubo fallas de conversión, el Ok value es un vec de bytes de tipo:
/// <len=9+X><id=7><index><begin><block>; tal que cada uno mide:
/// <4bytes><1byte><4bytes><4bytes><Xbytes>
fn encode_piece(
    piece_index: u32,
    beginning_byte_index: u32,
    block: Vec<u8>,
) -> Result<Vec<u8>, P2PMessageError> {
    let length_prefix = if let Ok(block_len) = u32::try_from(block.len()) {
        9 + block_len
    } else {
        return Err(P2PMessageError::FromUsizeToU32Error);
    };

    let length_prefix_as_bytes = length_prefix.to_be_bytes().to_vec();
    let piece_index_as_bytes = piece_index.to_be_bytes().to_vec();
    let byte_index_as_bytes = beginning_byte_index.to_be_bytes().to_vec();

    let mut encoded_piece = length_prefix_as_bytes;
    encoded_piece.push(ID_PIECE);

    piece_index_as_bytes
        .iter()
        .for_each(|byte| encoded_piece.push(*byte));

    byte_index_as_bytes
        .iter()
        .for_each(|byte| encoded_piece.push(*byte));

    block.iter().for_each(|byte| encoded_piece.push(*byte));

    Ok(encoded_piece)
}

///
/// Devuelve un vec de bytes de tipo:
/// <len=13><id=8><index><begin><length>; tal que cada uno mide:
/// <4bytes><1byte><4bytes><4bytes><4bytes>
fn encode_cancel(piece_index: u32, beginning_byte_index: u32, amount_of_bytes: u32) -> Vec<u8> {
    common_encode_for_request_and_cancel(
        piece_index,
        beginning_byte_index,
        amount_of_bytes,
        ID_CANCEL,
    )
}

///
/// Devuelve un vec de bytes de tipo:
/// <len=3><id=9><listen-port>; tal que cada uno mide:
/// <4bytes><1byte><2bytes>
fn encode_port(listen_port: u32) -> Vec<u8> {
    let length_prefix_as_bytes = vec![0u8, 0, 0, 3];
    let listen_port_as_bytes = listen_port.to_be_bytes().to_vec();

    let mut encoded_port = length_prefix_as_bytes;
    encoded_port.push(ID_PORT);

    listen_port_as_bytes
        .iter()
        .enumerate()
        .for_each(|(i, byte)| {
            if i >= 2 {
                //ya que el listen-port solo debe ser de 2 bytes
                encoded_port.push(*byte)
            }
        });

    encoded_port
}

///
/// Si el protocolo usado es BitTorrent protocol, el Ok value es un vec de bytes de tipo:
/// <pstrlen><pstr><reserved><info_hash><peer_id>;  tal que cada uno mide:
/// <1byte><19bytes><8bytes><20bytes><20bytes>
fn encode_handshake(
    protocol_str: String,
    info_hash: Vec<u8>,
    peer_id: String,
) -> Result<Vec<u8>, P2PMessageError> {
    if protocol_str != PSTR_STRING_HANDSHAKE {
        return Err(P2PMessageError::InvalidProtocolStrError);
    }
    let protocol_str_len = PSTRLEN_VALUE_HANDSHAKE; // Específico de protocolo BitTorrent
    let reserved_bytes = [0u8; 8];

    let mut encoded_handshake = vec![protocol_str_len];

    protocol_str
        .bytes()
        .for_each(|byte| encoded_handshake.push(byte));

    reserved_bytes
        .iter()
        .for_each(|byte| encoded_handshake.push(*byte));

    info_hash
        .iter()
        .for_each(|byte| encoded_handshake.push(*byte));

    peer_id
        .bytes()
        .for_each(|byte| encoded_handshake.push(byte));

    Ok(encoded_handshake)
}

/// Codifica un P2PMessage a su correspondiente representación en bytes para su envío.
///
/// A partir de dicho P2PMessage, devuelve un Result tal que:
///
/// - El Ok value es un vector de bytes correspondientes al mensaje para ser enviado a otro peer.
/// - El Err value es un P2PMessageError según sea el caso.
///
/// ## Notas importantes según mensaje:
///
/// - ***KeepAlive, Choke, Unchoke, Interested, Not interested, Have, Request, Cancel, Port***: En estos casos
///   NUNCA se va a obtener por retorno la variante Err.
///
/// - ***Bitfield***: Antes de realizar su encoding, verificar que la longitud del vector de PieceStatuses que se
///   le pasa sea la misma que la cantidad de piezas a descargar informada en el diccionario de clave info del '.torrent'.
///
/// - ***Request, Piece, Cancel***: Tanto la cantidad de bytes que se piden en la Request y el tamaño del
///   bloque suelen ser de 2^14 (16KB) por sugerencia de la documentación. Por ende la indexación se puede realizar
///   por medio de multiplos de tal numero (Ver ejemplo de uso más abajo).
///
/// ### Puede presentarse variante Err cuando:
///
/// - ***Piece*** : El length prefix del mensaje requiere ser de 4 bytes == un u32. Dicho prefix es calculado como 1 + la longitud
///   del block a enviar. Como la longitud del block es un usize, entonces se requiere una conversion que puede fallar. Por esto puede
///   devolver Err.
///
/// - ***Bitfield*** : Similar a Piece, al encodearlo se convierte un usize (del largo del payload en bytes) a u32 para calcular el
///   length prefix.
///
/// - ***Handshake***: Si bien el protocol_str se pasa al crear una variante de tipo Handshake, para propósitos exclusivos de este proyecto
///   al intentar encodear el mensaje dicho string TIENE que ser "BitTorrent protocol". Si esto no se cumple entonces
///   el valor de retorno es la variante Err. (Esto es para evitar posibles fallas involuntarias).
///
/// # Ejemplo de uso:
///
/// ```no_run
/// # use fa_torrent::torrent::parsers::p2p;
/// # use fa_torrent::torrent::parsers::p2p::message::*;
/// // Crea un mensaje Request en el que pide la pieza de indice 3, de la cual pide a
/// // partir el byte de indice 2*(...), y solucita un bloque de longitud 16KB:
/// let request = P2PMessage::Request {piece_index: 3, beginning_byte_index: 2*(2u32.pow(14)) , amount_of_bytes: 2u32.pow(14)};
/// let request_bytes = p2p::encoder::to_bytes(request);
/// // ... para posteriormente neviarle estos bytes a otro peer a traves de un socket.
/// ```
///
pub fn to_bytes(p2p_msg: P2PMessage) -> Result<Vec<u8>, P2PMessageError> {
    match p2p_msg {
        P2PMessage::KeepAlive => Ok(encode_keep_alive()),
        P2PMessage::Choke => Ok(encode_choke()),
        P2PMessage::Unchoke => Ok(encode_unchoke()),
        P2PMessage::Interested => Ok(encode_interested()),
        P2PMessage::NotInterested => Ok(encode_not_interested()),
        P2PMessage::Have { piece_index } => Ok(encode_have(piece_index)),
        P2PMessage::Bitfield { bitfield } => encode_bitfield(bitfield),
        P2PMessage::Request {
            piece_index,
            beginning_byte_index,
            amount_of_bytes,
        } => Ok(encode_request(
            piece_index,
            beginning_byte_index,
            amount_of_bytes,
        )),
        P2PMessage::Piece {
            piece_index,
            beginning_byte_index,
            block,
        } => encode_piece(piece_index, beginning_byte_index, block),
        P2PMessage::Cancel {
            piece_index,
            beginning_byte_index,
            amount_of_bytes,
        } => Ok(encode_cancel(
            piece_index,
            beginning_byte_index,
            amount_of_bytes,
        )),
        P2PMessage::Port { listen_port } => Ok(encode_port(listen_port)),
        P2PMessage::Handshake {
            protocol_str,
            info_hash,
            peer_id,
        } => encode_handshake(protocol_str, info_hash, peer_id),
    }
}

#[cfg(test)]
mod tests_p2p_encoder {

    use super::*;

    mod tests_payloadless_encodings {
        use super::*;

        #[test]
        fn encode_keep_alive_ok() {
            assert_eq!(Ok(vec![0; 4]), to_bytes(P2PMessage::KeepAlive))
        }

        #[test]
        fn encode_choke_ok() {
            assert_eq!(Ok(vec![0, 0, 0, 1, ID_CHOKE]), to_bytes(P2PMessage::Choke))
        }

        #[test]
        fn encode_unchoke_ok() {
            assert_eq!(
                Ok(vec![0, 0, 0, 1, ID_UNCHOKE]),
                to_bytes(P2PMessage::Unchoke)
            )
        }

        #[test]
        fn encode_interested_ok() {
            assert_eq!(
                Ok(vec![0, 0, 0, 1, ID_INTERESTED]),
                to_bytes(P2PMessage::Interested)
            )
        }

        #[test]
        fn encode_not_interested_ok() {
            assert_eq!(
                Ok(vec![0, 0, 0, 1, ID_NOT_INTERESTED]),
                to_bytes(P2PMessage::NotInterested)
            )
        }
    }

    mod tests_have_encoding {
        use super::*;

        #[test]
        fn encode_have_with_low_piece_index_ok() {
            let msg_to_send = P2PMessage::Have { piece_index: 2 };
            let expected_bytes = vec![0, 0, 0, 5, ID_HAVE, 0, 0, 0, 2];
            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }

        #[test]
        fn encode_have_with_high_piece_index_ok() {
            let msg_to_send = P2PMessage::Have {
                piece_index: 847249419,
            };
            let expected_bytes = vec![0, 0, 0, 5, ID_HAVE, 0b00110010, 0b10000000, 0, 0b00001011];
            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }
    }

    mod tests_bitfield_encoding {
        use super::*;

        #[test]
        fn encode_bitfield_that_should_set_least_significant_bits_on_single_byte_ok() {
            let msg_to_send = P2PMessage::Bitfield {
                bitfield: vec![
                    PieceStatus::MissingPiece,
                    PieceStatus::MissingPiece,
                    PieceStatus::MissingPiece,
                    PieceStatus::MissingPiece,
                    PieceStatus::MissingPiece,
                    PieceStatus::MissingPiece,
                    PieceStatus::ValidAndAvailablePiece,
                    PieceStatus::ValidAndAvailablePiece,
                ],
            };
            let expected_bytes = vec![0, 0, 0, 2, ID_BITFIELD, 0b00000011];

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }

        #[test]
        fn encode_bitfield_that_should_set_most_significant_bits_on_single_byte_ok() {
            let msg_to_send = P2PMessage::Bitfield {
                bitfield: vec![
                    PieceStatus::ValidAndAvailablePiece,
                    PieceStatus::ValidAndAvailablePiece,
                    PieceStatus::ValidAndAvailablePiece,
                    PieceStatus::ValidAndAvailablePiece,
                    PieceStatus::ValidAndAvailablePiece,
                    PieceStatus::MissingPiece,
                    PieceStatus::MissingPiece,
                    PieceStatus::MissingPiece,
                ],
            };
            let expected_bytes = vec![0, 0, 0, 2, ID_BITFIELD, 0b11111000];

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }

        #[test]
        fn encode_bitfield_that_should_set_some_bits_and_should_have_spare_trailing_bits_on_single_byte_ok(
        ) {
            // Es decir, cuando la cantidad de piezas no es multiplo de 8, el encoding deberia completar los ultimos bits del resultado sobrantes con ceros; <-- para el caso en el que solo se deberia obtener un byte.
            let msg_to_send = P2PMessage::Bitfield {
                bitfield: vec![
                    PieceStatus::ValidAndAvailablePiece,
                    PieceStatus::ValidAndAvailablePiece,
                    PieceStatus::MissingPiece,
                    PieceStatus::ValidAndAvailablePiece,
                ],
            };
            let expected_bytes = vec![0, 0, 0, 2, ID_BITFIELD, 0b11010000];

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }

        // funcion de ayuda para los dos test siguientes
        fn build_multiple_bytes_bitfield() -> Vec<PieceStatus> {
            let mut result = vec![PieceStatus::ValidAndAvailablePiece; 9];
            let mut aux_vec = vec![PieceStatus::MissingPiece; 6];
            aux_vec.push(PieceStatus::ValidAndAvailablePiece);

            result.append(&mut aux_vec);
            aux_vec = vec![PieceStatus::MissingPiece; 8];
            result.append(&mut aux_vec);

            result
        }

        #[test]
        fn encode_bitfield_of_multiple_bytes_given_enough_piece_statuses_to_fill_all_resulting_bytes_ok(
        ) {
            let bitfield_info = build_multiple_bytes_bitfield();

            let msg_to_send = P2PMessage::Bitfield {
                bitfield: bitfield_info,
            };
            let expected_bytes = vec![0, 0, 0, 4, ID_BITFIELD, 0b11111111, 0b10000001, 0b00000000];

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }

        #[test]
        fn encode_bitfield_of_multiple_bytes_given_not_enough_piece_statuses_to_fill_all_resulting_bytes_ok(
        ) {
            // Es decir, cuando la cantidad de piezas no es multiplo de 8, el encoding deberia completar los ultimos bits del resultado sobrantes con ceros; <-- para el caso general en el que se obtienen multiples bytes.
            let mut bitfield_info = build_multiple_bytes_bitfield();
            for _i in 0..10 {
                bitfield_info.pop();
            }

            let msg_to_send = P2PMessage::Bitfield {
                bitfield: bitfield_info,
            };
            let expected_bytes = vec![0, 0, 0, 3, ID_BITFIELD, 0b11111111, 0b10000000];

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }
    }

    mod tests_download_and_upload_related_encodings {
        use super::*;
        const DEFAULT_BLOCK_SIZE: u32 = 16384;

        #[test]
        fn encode_request_results_ok() {
            let msg_to_send = P2PMessage::Request {
                piece_index: 2,
                beginning_byte_index: 3 * (DEFAULT_BLOCK_SIZE),
                amount_of_bytes: DEFAULT_BLOCK_SIZE,
            };

            let mut byte_index = (3 * DEFAULT_BLOCK_SIZE).to_be_bytes().to_vec();
            let mut amount_of_bytes = DEFAULT_BLOCK_SIZE.to_be_bytes().to_vec();

            let mut expected_bytes = vec![0, 0, 0, 13, ID_REQUEST, 0, 0, 0, 2];
            expected_bytes.append(&mut byte_index);
            expected_bytes.append(&mut amount_of_bytes);

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }

        #[test]
        fn encode_piece_results_ok() {
            let mut default_data_block = vec![1u8; DEFAULT_BLOCK_SIZE as usize];
            let msg_to_send = P2PMessage::Piece {
                piece_index: 4,
                beginning_byte_index: 2 * DEFAULT_BLOCK_SIZE,
                block: default_data_block.clone(),
            };

            let mut byte_index = (2 * DEFAULT_BLOCK_SIZE).to_be_bytes().to_vec();

            let mut expected_bytes = vec![0, 0, 0b01000000, 9, ID_PIECE, 0, 0, 0, 4];
            expected_bytes.append(&mut byte_index);
            expected_bytes.append(&mut default_data_block);

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }

        #[test]
        fn encode_cancel_results_ok() {
            let msg_to_send = P2PMessage::Cancel {
                piece_index: 3,
                beginning_byte_index: 3 * (DEFAULT_BLOCK_SIZE),
                amount_of_bytes: DEFAULT_BLOCK_SIZE,
            };

            let mut byte_index = (3 * DEFAULT_BLOCK_SIZE).to_be_bytes().to_vec();
            let mut amount_of_bytes = DEFAULT_BLOCK_SIZE.to_be_bytes().to_vec();

            let mut expected_bytes = vec![0, 0, 0, 13, ID_CANCEL, 0, 0, 0, 3];
            expected_bytes.append(&mut byte_index);
            expected_bytes.append(&mut amount_of_bytes);

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }

        #[test]
        fn encode_port_results_ok() {
            let msg_to_send = P2PMessage::Port { listen_port: 6881 };

            let mut port_in_bytes = 6881u16.to_be_bytes().to_vec();

            let mut expected_bytes = vec![0, 0, 0, 3, ID_PORT];
            expected_bytes.append(&mut port_in_bytes);

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }
    }

    mod tests_handshake_encoding {
        use super::*;

        #[test]
        fn encode_handshake_ok() {
            let mut sha1d_info_hash = vec![1; 20];
            let mut peer_id_as_bytes: Vec<u8> = "-FA0001-012345678901".bytes().collect();
            let peer_id = String::from_utf8(peer_id_as_bytes.clone()).unwrap();
            let msg_to_send = P2PMessage::Handshake {
                protocol_str: PSTR_STRING_HANDSHAKE.to_owned(),
                info_hash: sha1d_info_hash.clone(),
                peer_id: peer_id.clone(),
            };

            let mut expected_bytes = vec![19];
            PSTR_STRING_HANDSHAKE
                .bytes()
                .for_each(|byte| expected_bytes.push(byte));
            [0u8; 8].iter().for_each(|byte| expected_bytes.push(*byte));
            expected_bytes.append(&mut sha1d_info_hash);
            expected_bytes.append(&mut peer_id_as_bytes);

            assert_eq!(Ok(expected_bytes), to_bytes(msg_to_send));
        }
    }
}
