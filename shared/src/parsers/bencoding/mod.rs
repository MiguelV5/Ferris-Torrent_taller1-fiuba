//! # Modulo de Bencoding
//! Este modulo va a servir para encodear y desencodear distintos tipos [Strings, Integers, Listas y Diccionarios]
//! al tipo bencoding que va a ser devuelto como String en caso de encodear y el tipo buscado en caso de desencodear

mod constants;
pub mod decoder;
pub mod encoder;
pub mod values;
