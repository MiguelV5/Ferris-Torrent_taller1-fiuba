//! # Modulo de Values
//! Este modulo contiene el enumerado con los valores utilizados en el Bencoding

///Enumerado de los distintos tipos que puede haber en el bencoding
use std::collections::HashMap;

#[derive(PartialEq, Debug, Clone)]
pub enum ValuesBencoding {
    String(String),
    Integer(i64),
    List(Vec<ValuesBencoding>),
    Dic(HashMap<String, ValuesBencoding>),
}
