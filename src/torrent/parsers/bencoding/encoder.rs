//! # Modulo de encoder de Bencoding
//! Este Modulo va a servir para pasar un String/Integer/List/Dic al formato Bencoding
//!  el cual sera representado por un String
#![allow(dead_code)]
use super::constants::*;
use super::values::ValuesBencoding;
use std::collections::HashMap;

///Esta funcion devuelve un String en el formato Bencoding
///  del String que se le haya pasado
pub fn from_string(to_bencode: String) -> String {
    let mut bencoding = String::new();
    let long_number = to_bencode.len() as u32;
    let long_str = long_number.to_string();

    bencoding.push_str(&long_str);
    bencoding.push(TWO_POINTS);
    bencoding.push_str(&to_bencode);

    bencoding
}

///Esta funcion devuelve un String del formato Bencoding del integer pasado
pub fn from_integer(to_bencode: i64) -> String {
    let mut bencoding = String::from(CHAR_I);
    let num_string = to_bencode.to_string();
    bencoding.push_str(&num_string);
    bencoding.push(CHAR_E);

    bencoding
}

///Esta funcion devuelve un String del formato Bencoding de la lista ([Vec]) pasada
pub fn from_list(to_bencode: Vec<ValuesBencoding>) -> String {
    let mut bencoding = String::from(CHAR_L);
    let iterator = to_bencode.into_iter();

    for values in iterator {
        let str_to_add = match values {
            ValuesBencoding::String(str) => from_string(str),
            ValuesBencoding::Integer(int) => from_integer(int),
            ValuesBencoding::List(list) => from_list(list),
            ValuesBencoding::Dic(dic) => from_dic(dic),
        };
        bencoding.push_str(&str_to_add);
    }

    bencoding.push(CHAR_E);
    bencoding
}

///Esta funcion devuelve un String del formato Bencoding de el Diccionario ([HashMap]) pasado
pub fn from_dic(to_bencode: HashMap<String, ValuesBencoding>) -> String {
    let mut bencoding = String::from(CHAR_D);

    for (key, value) in to_bencode.into_iter() {
        bencoding.push_str(&from_string(key));

        let str_to_add = match value {
            ValuesBencoding::String(str) => from_string(str),
            ValuesBencoding::Integer(int) => from_integer(int),
            ValuesBencoding::List(list) => from_list(list),
            ValuesBencoding::Dic(dic) => from_dic(dic),
        };
        bencoding.push_str(&str_to_add);
    }

    bencoding.push(CHAR_E);
    bencoding
}

#[cfg(test)]
mod tests {
    use super::*;
    mod tests_from_string {
        use super::*;
        #[test]
        fn from_string_create_ok() {
            let to_bencode = String::from("Test");
            let result_expected = String::from("4:Test");
            assert_eq!(result_expected, from_string(to_bencode));

            let to_bencode = String::from("Interstellar");
            let result_expected = String::from("12:Interstellar");
            assert_eq!(result_expected, from_string(to_bencode));

            let to_bencode = String::from("");
            let result_expected = String::from("0:");
            assert_eq!(result_expected, from_string(to_bencode));
        }
    }
    mod tests_from_integer {
        use super::*;
        #[test]
        fn from_integer_create_positive_ok() {
            let number = 5;
            let bencoding_expected = String::from("i5e");
            assert_eq!(bencoding_expected, from_integer(number));

            let number = 276498;
            let bencoding_expected = String::from("i276498e");
            assert_eq!(bencoding_expected, from_integer(number));

            let number = 11234985784903;
            let bencoding_expected = String::from("i11234985784903e");
            assert_eq!(bencoding_expected, from_integer(number));
        }
        #[test]
        fn from_integer_create_negative_ok() {
            let number = -9;
            let bencoding_expected = String::from("i-9e");
            assert_eq!(bencoding_expected, from_integer(number));

            let number = -2349874;
            let bencoding_expected = String::from("i-2349874e");
            assert_eq!(bencoding_expected, from_integer(number));

            let number = -109843209420938;
            let bencoding_expected = String::from("i-109843209420938e");
            assert_eq!(bencoding_expected, from_integer(number));
        }
        #[test]
        fn from_integer_create_zero_ok() {
            let number = 0;
            let bencoding_expected = String::from("i0e");
            assert_eq!(bencoding_expected, from_integer(number));

            let number = -0;
            let bencoding_expected = String::from("i0e");
            assert_eq!(bencoding_expected, from_integer(number));
        }
    }
    mod tests_from_list {
        use super::*;
        #[test]
        fn from_list_create_ok() {
            let str_list = ValuesBencoding::String("Init".to_owned());
            let int_list = ValuesBencoding::Integer(123);
            let list = vec![str_list, int_list];
            let expected_bencoding = String::from("l4:Initi123ee");

            assert_eq!(expected_bencoding, from_list(list));
        }
        #[test]
        fn from_list_create_with_list_inside_ok() {
            let str_list = ValuesBencoding::String("Init".to_owned());
            let int_list = ValuesBencoding::Integer(123);
            let list = vec![str_list, int_list];

            let str_list = ValuesBencoding::String("Fin".to_owned());
            let int_list = ValuesBencoding::Integer(-125);
            let list_inside = vec![int_list, ValuesBencoding::List(list), str_list];

            let expected_bencoding = String::from("li-125el4:Initi123ee3:Fine");

            assert_eq!(expected_bencoding, from_list(list_inside));
        }
    }
    mod tests_from_dic {
        use crate::torrent::parsers::bencoding;

        use super::*;
        #[test]
        fn from_dic_create_ok() {
            let mut dic = HashMap::new();
            let rest = String::from("");
            dic.insert("A".to_owned(), ValuesBencoding::String("Meta".to_owned()));
            dic.insert("B".to_owned(), ValuesBencoding::Integer(-125));
            dic.insert("C".to_owned(), ValuesBencoding::Integer(0));
            dic.insert("D".to_owned(), ValuesBencoding::String("Fin".to_owned()));

            let bencoding = from_dic(dic.clone());

            assert_eq!(bencoding::decoder::to_dic(bencoding), Ok((dic, rest)));
        }
    }
}
