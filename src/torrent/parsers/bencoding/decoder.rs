//!# Modulo de decoder de Bencoding
//! Este modulo va a servir para pasar a String/Integer/List/Dic dado un String
//!  que esta en el formato Bencoding

use super::constants::*;
use super::values::*;
use std::collections::HashMap;

type DicValues = HashMap<Vec<u8>, ValuesBencoding>;
type TupleStringRest = (Vec<u8>, Vec<u8>);
type TupleIntegerRest = (i64, Vec<u8>);
type TupleListRest = (Vec<ValuesBencoding>, Vec<u8>);
type TupleValueRest = (ValuesBencoding, Vec<u8>);
type TupleDicRest = (DicValues, Vec<u8>);

const NEGATIVE_ZERO: &str = "-0";
const MINUS: char = '-';
const ZERO: char = '0';

type ResultBencoding<T> = Result<T, ErrorBencoding>;

///Funcion que dado un String en formato bencoding va a desencodear a String y luego va a devolver un Result
/// de una tupla con el String desencodeado y lo que sobre del String pasado, o en caso de error se devolvera
/// el mismo que sera del tipo ErrorBencoding, por ej: en caso de pasar "4:testi32e3:fin" se devolvera Ok con la tupla
/// ("test", "i32e3:fin")
fn to_string(to_parse: Vec<u8>) -> ResultBencoding<TupleStringRest> {
    let mut result = vec![];
    let mut long_string = String::new();
    let mut valid_format = false;

    //Tomo todos los valores antes del ':' que deberian representar el largo del string
    let mut list_chars = to_parse.into_iter();
    for long_char in list_chars.by_ref() {
        if long_char == TWO_POINTS {
            valid_format = true;
            break;
        }
        long_string.push(long_char as char);
    }

    //Valido que haya pasado por un ':' al recorrer el string
    if !valid_format {
        return Err(ErrorBencoding::String(ErrorType::Format));
    }

    //Parseo el numero en string pasandolo a u32
    let long_int = match long_string.parse::<u32>() {
        Ok(number) => number,
        Err(_) => return Err(ErrorBencoding::String(ErrorType::Format)),
    };

    //Voy concatenando caracter a caracter formando el string con la longitud que tome anteriormente
    for _ in 0..long_int {
        if let Some(char_string) = list_chars.next() {
            result.push(char_string);
        } else {
            return Err(ErrorBencoding::String(ErrorType::Long));
        }
    }

    Ok((result, list_chars.collect()))
}

fn is_valid_number(num: String) -> bool {
    if num == NEGATIVE_ZERO {
        return false;
    }
    let mut chars_num = num.chars();

    if let Some(digit) = chars_num.next() {
        if digit == MINUS {
            return is_valid_number(chars_num.collect());
        } else if digit == ZERO && chars_num.next().is_some() {
            return false;
        }
    }
    true
}

///Funcion que va a pasar un String en formato bencoding a un i64, los cual va a devolverlos en un Result, con el
/// formato de una tupla, la cual su primer valor sera el i64 y el siguiente el resto del string del bencoding pasado,
/// en caso de error se devolvera el mismo
fn to_integer(to_parse: Vec<u8>) -> ResultBencoding<TupleIntegerRest> {
    let mut num_str = String::new();
    let mut valid_format = false;
    let mut list_chars = to_parse.into_iter();

    //Valido que el primer caracter sea 'i'
    if let Some(CHAR_I) = list_chars.next() {
    } else {
        return Err(ErrorBencoding::Integer(ErrorType::Format));
    }

    for num_char in list_chars.by_ref() {
        if num_char == CHAR_E {
            valid_format = true;
            break;
        }
        num_str.push(num_char as char);
    }

    //Valido que haya terminado en 'e'
    if !valid_format {
        return Err(ErrorBencoding::Integer(ErrorType::Format));
    }
    //Valido que el valor del numero sea valido
    if !is_valid_number(num_str.clone()) {
        return Err(ErrorBencoding::Integer(ErrorType::Number));
    }

    match num_str.parse::<i64>() {
        Ok(num) => Ok((num, list_chars.collect())),
        Err(_) => Err(ErrorBencoding::Integer(ErrorType::Number)),
    }
}

fn take_value_by_type(
    from: u8,
    type_char: u8,
    to_parse: Vec<u8>,
) -> ResultBencoding<TupleValueRest> {
    if type_char.is_ascii_digit() {
        let (str, next_parse) = to_string(to_parse)?;
        Ok((ValuesBencoding::String(str), next_parse))
    } else if type_char == CHAR_I {
        let (int, next_parse) = to_integer(to_parse)?;
        Ok((ValuesBencoding::Integer(int), next_parse))
    } else if type_char == CHAR_L {
        let (list, next_parse) = to_list(to_parse)?;
        Ok((ValuesBencoding::List(list), next_parse))
    } else if type_char == CHAR_D {
        let (dic, next_parse) = to_dic(to_parse)?;
        Ok((ValuesBencoding::Dic(dic), next_parse))
    } else if from == CHAR_L {
        Err(ErrorBencoding::List(ErrorType::Format))
    } else {
        Err(ErrorBencoding::Dic(ErrorType::Format))
    }
}

///Funcion que va a desencodear un String del tipo Bencoding en una lista ([Vec]), la cual sera devuelta en un Result con
/// el formato de una tupla en la cual su primer valor sera la lista desencodeada y su segundo valor sera el restante del String,
/// en caso de error se devolvera el mismo
fn to_list(to_parse: Vec<u8>) -> ResultBencoding<TupleListRest> {
    let mut list_return = Vec::new();
    let mut valid_format = false;
    let mut list_chars = to_parse.into_iter();

    //Reviso que el string comience con 'l'
    match list_chars.next() {
        Some(CHAR_L) => (),
        _ => return Err(ErrorBencoding::List(ErrorType::Format)),
    }

    let mut to_parse: Vec<u8> = list_chars.clone().collect();

    while let Some(next_char) = list_chars.next() {
        if next_char == CHAR_E {
            valid_format = true;
            break;
        }
        let (value, next_parse) = take_value_by_type(CHAR_L, next_char, to_parse)?;
        list_return.push(value);
        to_parse = next_parse;
        list_chars = to_parse.clone().into_iter();
    }

    if !valid_format {
        return Err(ErrorBencoding::List(ErrorType::Format));
    }

    Ok((list_return, list_chars.collect()))
}

///Funcion para desencodear un String del tipo bencoding en formato de diccionario ([HashMap]) en el cual se devolvera un Result,
/// el cual contendra una tupla con el diccionario como primer valor y el sobrante del string del bencoding pasado como segundo
/// valor, en caso de error se devolvera el correspondiente
pub fn to_dic(to_parse: Vec<u8>) -> ResultBencoding<TupleDicRest> {
    let mut dic_return = HashMap::new();
    let mut valid_format = false;
    let mut list_chars = to_parse.into_iter();

    //Reviso que el string comience con 'd'
    match list_chars.next() {
        Some(CHAR_D) => (),
        _ => return Err(ErrorBencoding::Dic(ErrorType::Format)),
    }

    let mut to_parse: Vec<u8> = list_chars.clone().collect();

    while let Some(next_char) = list_chars.next() {
        if next_char == CHAR_E {
            valid_format = true;
            break;
        }
        let (key, next_parse) = match to_string(to_parse.clone()) {
            Ok((k, p)) => (k, p),
            Err(_) => return Err(ErrorBencoding::Dic(ErrorType::Format)),
        };
        if let Some(char_next) = next_parse.clone().into_iter().next() {
            let (value, next_parse) = take_value_by_type(CHAR_D, char_next, next_parse)?;
            dic_return.insert(key, value);
            to_parse = next_parse;
        } else {
            return Err(ErrorBencoding::Dic(ErrorType::Format));
        }

        list_chars = to_parse.clone().into_iter();
    }

    if !valid_format {
        return Err(ErrorBencoding::Dic(ErrorType::Format));
    }

    Ok((dic_return, list_chars.collect()))
}

pub fn from_torrent_to_dic(torrent_file: Vec<u8>) -> Result<DicValues, ErrorBencoding> {
    match to_dic(torrent_file) {
        Ok((result, rest)) => {
            if rest.is_empty() {
                Ok(result)
            } else {
                Err(ErrorBencoding::Dic(ErrorType::Format))
            }
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod tests_to_strings {
        use super::*;
        #[test]
        fn to_string_ok() {
            let bencoding_string = String::from("3:exe");
            let bencoding_bytes = bencoding_string.as_bytes().to_vec();
            let return_str = String::from("exe").as_bytes().to_vec();
            let return_rest = vec![];

            let result = to_string(bencoding_bytes);
            assert!(result.is_ok());
            let (str, rest) = result.unwrap();
            assert_eq!(str, return_str);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_string_ok_rest_valid() {
            let bencoding_string = String::from("5:magic4:testi32e");
            let bencoding_bytes = bencoding_string.as_bytes().to_vec();
            let return_str = "magic".as_bytes().to_vec();
            let return_rest = "4:testi32e".as_bytes().to_vec();

            let result = to_string(bencoding_bytes);
            assert!(result.is_ok());

            let (str, rest) = result.unwrap();
            assert_eq!(str, return_str);
            assert_eq!(rest, return_rest);

            let return_str = "test".as_bytes().to_vec();
            let return_rest = "i32e".as_bytes().to_vec();

            let result = to_string(rest);
            assert!(result.is_ok());

            let (str, rest) = result.unwrap();
            assert_eq!(str, return_str);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_string_error_format() {
            let bencoding = "4exe".as_bytes().to_vec();
            assert_eq!(
                to_string(bencoding),
                Err(ErrorBencoding::String(ErrorType::Format))
            );
        }
        #[test]
        fn to_string_error_without_number() {
            let bencoding = "test".as_bytes().to_vec();
            assert_eq!(
                to_string(bencoding),
                Err(ErrorBencoding::String(ErrorType::Format))
            );
        }
        #[test]
        fn to_string_error_invalid_number() {
            let bencoding = "a:test".as_bytes().to_vec();
            assert_eq!(
                to_string(bencoding),
                Err(ErrorBencoding::String(ErrorType::Format))
            );
        }
        #[test]
        fn to_string_error_invalid_long() {
            let bencoding = "12:test".as_bytes().to_vec();
            assert_eq!(
                to_string(bencoding),
                Err(ErrorBencoding::String(ErrorType::Long))
            );
        }
    }
    mod tests_to_integers {
        use super::*;
        #[test]
        fn to_integer_ok_positive() {
            let bencoding_int = "i32e".as_bytes().to_vec();
            let return_int = 32;
            let return_rest = vec![];

            let result = to_integer(bencoding_int);
            assert!(result.is_ok());

            let (int, rest) = result.unwrap();
            assert_eq!(int, return_int);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_integer_ok_negative() {
            let bencoding_int = "i-320e".as_bytes().to_vec();
            let return_int = -320;
            let return_rest = vec![];

            let result = to_integer(bencoding_int);
            assert!(result.is_ok());

            let (int, rest) = result.unwrap();
            assert_eq!(int, return_int);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_integer_ok_rest_valid() {
            let bencoding_int = "i32ei-200e4:test".as_bytes().to_vec();
            let return_int = 32;
            let return_rest = "i-200e4:test".as_bytes().to_vec();

            let result = to_integer(bencoding_int);
            assert!(result.is_ok());

            let (str, rest) = result.unwrap();
            assert_eq!(str, return_int);
            assert_eq!(rest, return_rest);

            let return_int = -200;
            let return_rest = "4:test".as_bytes().to_vec();

            let result = to_integer(rest);
            assert!(result.is_ok());

            let (str, rest) = result.unwrap();
            assert_eq!(str, return_int);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_integer_error_format() {
            let bencoding_int = "32e".as_bytes().to_vec();
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Format))
            );

            let bencoding_int = "i32".as_bytes().to_vec();
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Format))
            );
        }
        #[test]
        fn to_integer_error_minus_zero() {
            let bencoding_int = "i-0e".as_bytes().to_vec();
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );
        }
        #[test]
        fn to_integer_error_zero_and_number() {
            let bencoding_int = "i018e".as_bytes().to_vec();
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );

            let bencoding_int = "i-08e".as_bytes().to_vec();
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );
        }
        #[test]
        fn to_integer_error_invalid_number() {
            let bencoding_int = "i2a3e".as_bytes().to_vec();
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );
        }
    }
    mod tests_to_lists {
        use super::*;
        #[test]
        fn to_list_ok() {
            let str_expected = ValuesBencoding::String("test".as_bytes().to_vec());
            let int_expected = ValuesBencoding::Integer(32);
            let rest_expected = "3:exe".as_bytes().to_vec();
            let result_expected = (vec![str_expected, int_expected], rest_expected);
            let to_parse = "l4:testi32ee3:exe".as_bytes().to_vec();
            let result = to_list(to_parse);
            assert_eq!(result, Ok(result_expected));
        }
        #[test]
        fn to_list_inside_list_ok() {
            let str_expected = ValuesBencoding::String("test".as_bytes().to_vec());
            let int_expected = ValuesBencoding::Integer(32);
            let vec_expected = ValuesBencoding::List(vec![str_expected, int_expected]);
            let rest_expected = "3:exe".as_bytes().to_vec();
            let result_expected = (vec![vec_expected], rest_expected);
            let to_parse = "ll4:testi32eee3:exe".as_bytes().to_vec();
            let result = to_list(to_parse);
            assert_eq!(result, Ok(result_expected));
        }
        #[test]
        fn to_list_error_format() {
            let to_parse = "4:testi32ee3:exe".as_bytes().to_vec();
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::List(ErrorType::Format))
            );

            let to_parse = "la:testi32ee3:exe".as_bytes().to_vec();
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::List(ErrorType::Format))
            );
        }
        #[test]
        fn to_list_error_not_close() {
            let to_parse = "l4:testi32e3:exe".as_bytes().to_vec();
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::List(ErrorType::Format))
            );
        }
        #[test]
        fn to_list_error_string() {
            let to_parse = "l4teste".as_bytes().to_vec();
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::String(ErrorType::Format))
            );

            let to_parse = "l10:teste".as_bytes().to_vec();
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::String(ErrorType::Long))
            );
        }
        #[test]
        fn to_list_error_integer() {
            let to_parse = "li-0ee".as_bytes().to_vec();
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );

            let to_parse = "li032ee".as_bytes().to_vec();
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );

            let to_parse = "li5".as_bytes().to_vec();
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::Integer(ErrorType::Format))
            );
        }
    }
    mod tests_to_dic {
        use super::*;
        #[test]
        fn to_dic_create_ok() {
            let bencoding = "d8:announcei32e4:test3:exee3:exe".as_bytes().to_vec();
            let mut dic_expected = HashMap::new();
            dic_expected.insert("announce".as_bytes().to_vec(), ValuesBencoding::Integer(32));
            dic_expected.insert(
                "test".as_bytes().to_vec(),
                ValuesBencoding::String("exe".as_bytes().to_vec()),
            );
            let rest_expected = "3:exe".as_bytes().to_vec();

            assert_eq!(Ok((dic_expected, rest_expected)), to_dic(bencoding));
        }
        #[test]
        fn to_dic_create_with_list_inside_ok() {
            let bencoding = "d8:announceli32ei-12ei0e4:abcde4:test3:exee3:exe"
                .as_bytes()
                .to_vec();
            let mut dic_expected = HashMap::new();
            let list = vec![
                ValuesBencoding::Integer(32),
                ValuesBencoding::Integer(-12),
                ValuesBencoding::Integer(0),
                ValuesBencoding::String("abcd".as_bytes().to_vec()),
            ];
            dic_expected.insert("announce".as_bytes().to_vec(), ValuesBencoding::List(list));
            dic_expected.insert(
                "test".as_bytes().to_vec(),
                ValuesBencoding::String("exe".as_bytes().to_vec()),
            );
            let rest_expected = "3:exe".as_bytes().to_vec();

            assert_eq!(Ok((dic_expected, rest_expected)), to_dic(bencoding));
        }
        #[test]
        fn to_dic_create_with_dic_inside_ok() {
            let bencoding = "d8:announced4:abcdi32ee4:test3:exee3:exe"
                .as_bytes()
                .to_vec();
            let mut dic_expected = HashMap::new();
            let mut dic = HashMap::new();
            dic.insert("abcd".as_bytes().to_vec(), ValuesBencoding::Integer(32));
            dic_expected.insert("announce".as_bytes().to_vec(), ValuesBencoding::Dic(dic));
            dic_expected.insert(
                "test".as_bytes().to_vec(),
                ValuesBencoding::String("exe".as_bytes().to_vec()),
            );
            let rest_expected = "3:exe".as_bytes().to_vec();

            assert_eq!(Ok((dic_expected, rest_expected)), to_dic(bencoding));
        }
        #[test]
        fn to_dic_create_complex_ok() {
            //Test mas complejo de tener un diccionario con una lista, un diccionario, un diccionario con listas
            //y una lista con diccionario
            let bencoding = "d4:listl1:A1:B1:Ci32ei0ee3:dicd1:Ai-125e1:Bi100e1:C3:fine8:dic_listd1:Ali1ei2ei3ee1:Bli-1ei-2ei-3eee8:list_dicld1:Ai32e1:Bi-125eeee".as_bytes().to_vec();

            let mut dic_expected = HashMap::new();

            let a = String::from("A");
            let b = String::from("B");
            let c = String::from("C");
            let list = vec![
                ValuesBencoding::String(a.clone().as_bytes().to_vec()),
                ValuesBencoding::String(b.clone().as_bytes().to_vec()),
                ValuesBencoding::String(c.clone().as_bytes().to_vec()),
                ValuesBencoding::Integer(32),
                ValuesBencoding::Integer(0),
            ];

            dic_expected.insert("list".as_bytes().to_vec(), ValuesBencoding::List(list));

            let mut dic = HashMap::new();

            dic.insert(
                a.clone().as_bytes().to_vec(),
                ValuesBencoding::Integer(-125),
            );
            dic.insert(b.clone().as_bytes().to_vec(), ValuesBencoding::Integer(100));
            dic.insert(
                c.clone().as_bytes().to_vec(),
                ValuesBencoding::String("fin".as_bytes().to_vec()),
            );

            dic_expected.insert("dic".as_bytes().to_vec(), ValuesBencoding::Dic(dic));

            let list1 = vec![
                ValuesBencoding::Integer(1),
                ValuesBencoding::Integer(2),
                ValuesBencoding::Integer(3),
            ];
            let list2 = vec![
                ValuesBencoding::Integer(-1),
                ValuesBencoding::Integer(-2),
                ValuesBencoding::Integer(-3),
            ];

            let mut dic_list = HashMap::new();
            dic_list.insert(a.clone().as_bytes().to_vec(), ValuesBencoding::List(list1));
            dic_list.insert(b.clone().as_bytes().to_vec(), ValuesBencoding::List(list2));

            dic_expected.insert(
                "dic_list".as_bytes().to_vec(),
                ValuesBencoding::Dic(dic_list),
            );

            let mut dic_in_list = HashMap::new();
            dic_in_list.insert(a.clone().as_bytes().to_vec(), ValuesBencoding::Integer(32));
            dic_in_list.insert(
                b.clone().as_bytes().to_vec(),
                ValuesBencoding::Integer(-125),
            );

            let list_dic = vec![ValuesBencoding::Dic(dic_in_list)];

            dic_expected.insert(
                "list_dic".as_bytes().to_vec(),
                ValuesBencoding::List(list_dic),
            );

            assert_eq!(Ok((dic_expected, vec![])), to_dic(bencoding))
        }
        #[test]
        fn to_dic_invalid_format() {
            let bencoding = "8:announcei32e4:test3:exee3:exe".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = "d8:announcei32e4:test3:exe3:exe".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = "d8:announcei32e4:test3:exe".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );
        }
        #[test]
        fn to_dic_invalid_key() {
            let bencoding = "di0ei32e4:test3:exee3:exe".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );
        }
        #[test]
        fn to_dic_invalid_num() {
            let bencoding = "d8:announcei-0e4:test3:exee3:exe".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::Integer(ErrorType::Number)),
                to_dic(bencoding)
            );
        }
        #[test]
        fn to_dic_invalid_list() {
            let bencoding = "d8:announcei32e4:testl2:el".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::List(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = "d8:announcei32e4:testlf:ele".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::List(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = "d8:announcei32e4:testl2:eli-0eee".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::Integer(ErrorType::Number)),
                to_dic(bencoding)
            );
        }
        #[test]
        fn to_dic_invalid_dic() {
            let bencoding = "d8:announcei32e4:testdi32ee".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = "d8:announcei32e4:testd3:exei-12e".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = "d8:announcei32e3:inid4:testi-0ee".as_bytes().to_vec();
            assert_eq!(
                Err(ErrorBencoding::Integer(ErrorType::Number)),
                to_dic(bencoding)
            );
        }
    }
}
