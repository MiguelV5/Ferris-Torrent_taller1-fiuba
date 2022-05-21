//! # Modulo de Values
//! Este modulo contiene el enumerado con los valores utilizados en el Bencoding

///Enumerado de los distintos tipos que puede haber en el bencoding
use std::{collections::HashMap, error::Error, fmt};

#[derive(PartialEq, Debug, Clone)]
pub enum ValuesBencoding {
    String(String),
    Integer(i64),
    List(Vec<ValuesBencoding>),
    Dic(HashMap<String, ValuesBencoding>),
}

///Enumerado de los distos tipos que pueden dar error con su descripcion de error dentro
#[derive(PartialEq, Debug)]
pub enum ErrorBencoding {
    String(ErrorType),
    Integer(ErrorType),
    List(ErrorType),
    Dic(ErrorType),
}

///Enumerado de los posibles errores al desencodear
#[derive(PartialEq, Debug)]
pub enum ErrorType {
    Format,
    Long,
    Number,
}

impl fmt::Display for ErrorBencoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {:?}", self)
    }
}

impl Error for ErrorBencoding {}
