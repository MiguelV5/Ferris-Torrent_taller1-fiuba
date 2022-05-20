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
    let mut keys_vector: Vec<&String> = to_bencode.keys().collect();
    keys_vector.sort();

    for key in keys_vector.into_iter() {
        bencoding.push_str(&from_string(key.clone()));

        if let Some(value) = to_bencode.get(key) {
            let str_to_add = match value {
                ValuesBencoding::String(str) => from_string(str.clone()),
                ValuesBencoding::Integer(int) => from_integer(*int),
                ValuesBencoding::List(list) => from_list(list.clone()),
                ValuesBencoding::Dic(dic) => from_dic(dic.clone()),
            };
            bencoding.push_str(&str_to_add);
        }
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
        use super::*;
        #[test]
        fn from_dic_create_ok() {
            let mut dic = HashMap::new();
            dic.insert("A".to_owned(), ValuesBencoding::String("Meta".to_owned()));
            dic.insert("B".to_owned(), ValuesBencoding::Integer(-125));
            dic.insert("C".to_owned(), ValuesBencoding::Integer(0));
            dic.insert("D".to_owned(), ValuesBencoding::String("Fin".to_owned()));

            let bencoding = from_dic(dic.clone());
            let expected_bencoding = String::from("d1:A4:Meta1:Bi-125e1:Ci0e1:D3:Fine");

            assert_eq!(bencoding, expected_bencoding);
        }
        #[test]
        fn from_dic_create_with_list_inside_ok() {
            let bencoding = String::from("d8:announceli32ei-12ei0e4:abcde4:test3:exee");
            let mut dic_to_bencode = HashMap::new();
            let list = vec![
                ValuesBencoding::Integer(32),
                ValuesBencoding::Integer(-12),
                ValuesBencoding::Integer(0),
                ValuesBencoding::String("abcd".to_owned()),
            ];
            dic_to_bencode.insert("announce".to_owned(), ValuesBencoding::List(list));
            dic_to_bencode.insert("test".to_owned(), ValuesBencoding::String("exe".to_owned()));

            assert_eq!(bencoding, from_dic(dic_to_bencode));
        }
        #[test]
        fn from_dic_create_with_dic_inside_ok() {
            let bencoding = String::from("d8:announced4:abcdi32ee4:test3:exee");
            let mut dic_to_bencode = HashMap::new();
            let mut dic = HashMap::new();
            dic.insert("abcd".to_owned(), ValuesBencoding::Integer(32));
            dic_to_bencode.insert("announce".to_owned(), ValuesBencoding::Dic(dic));
            dic_to_bencode.insert("test".to_owned(), ValuesBencoding::String("exe".to_owned()));

            assert_eq!(bencoding, from_dic(dic_to_bencode));
        }
        #[test]
        fn from_dic_create_complex_ok() {
            let bencoding = String::from("d3:dicd1:Ai-125e1:Bi100e1:C3:fine8:dic_listd1:Ali1ei2ei3ee1:Bli-1ei-2ei-3eee4:listl1:A1:B1:Ci32ei0ee8:list_dicld1:Ai32e1:Bi-125eeee");

            let mut dic_to_bencode = HashMap::new();

            let a = String::from("A");
            let b = String::from("B");
            let c = String::from("C");
            let list = vec![
                ValuesBencoding::String(a.clone()),
                ValuesBencoding::String(b.clone()),
                ValuesBencoding::String(c.clone()),
                ValuesBencoding::Integer(32),
                ValuesBencoding::Integer(0),
            ];

            dic_to_bencode.insert("list".to_owned(), ValuesBencoding::List(list));

            let mut dic = HashMap::new();

            dic.insert(a.clone(), ValuesBencoding::Integer(-125));
            dic.insert(b.clone(), ValuesBencoding::Integer(100));
            dic.insert(c.clone(), ValuesBencoding::String("fin".to_owned()));

            dic_to_bencode.insert("dic".to_owned(), ValuesBencoding::Dic(dic));

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
            dic_list.insert(a.clone(), ValuesBencoding::List(list1));
            dic_list.insert(b.clone(), ValuesBencoding::List(list2));

            dic_to_bencode.insert("dic_list".to_owned(), ValuesBencoding::Dic(dic_list));

            let mut dic_in_list = HashMap::new();
            dic_in_list.insert(a.clone(), ValuesBencoding::Integer(32));
            dic_in_list.insert(b.clone(), ValuesBencoding::Integer(-125));

            let list_dic = vec![ValuesBencoding::Dic(dic_in_list)];

            dic_to_bencode.insert("list_dic".to_owned(), ValuesBencoding::List(list_dic));

            assert_eq!(bencoding, from_dic(dic_to_bencode));
        }
    }
}
