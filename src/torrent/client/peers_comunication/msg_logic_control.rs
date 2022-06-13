//! # Modulo de logica de control para interaccion individual
//! Este modulo contiene las funciones encargadas de manejar la conexion y
//! comunicacion exclusivamente un peer indicado
//!

use core::fmt;
use std::error::Error;

pub const BLOCK_BYTES: u32 = 16384; //2^14 bytes

pub const SECS_READ_TIMEOUT: u64 = 120;
pub const NANOS_READ_TIMEOUT: u32 = 0;

#[derive(PartialEq, Debug, Clone)]
/// Representa un tipo de error en la comunicación general P2P con un peer individual.
pub enum MsgLogicControlError {
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
    UpdatingFields(String),
    CalculatingServerPeerIndex(String),
    CalculatingPieceLenght(String),
    SetUpDirectory(String),
}

impl fmt::Display for MsgLogicControlError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for MsgLogicControlError {}
