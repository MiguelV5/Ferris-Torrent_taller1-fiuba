use std::collections::HashMap;

type TupleStringRest = (String, String);
type TupleIntegerRest = (i64, String);
type TupleListRest = (Vec<ValuesBencoding>, String);
type TupleValueRest = (ValuesBencoding, String);
type TupleDicRest = (HashMap<String, ValuesBencoding>, String);

type ResultBencoding<T> = Result<T, ErrorBencoding>;

#[derive(PartialEq, Debug)]
pub enum ErrorBencoding {
    String(ErrorType),
    Integer(ErrorType),
    List(ErrorType),
    Dic(ErrorType),
}

#[derive(PartialEq, Debug)]
pub enum ErrorType {
    Format,
    Long,
    Number,
}

#[derive(PartialEq, Debug, Clone)]
pub enum ValuesBencoding {
    String(String),
    Integer(i64),
    List(Vec<ValuesBencoding>),
    Dic(HashMap<String, ValuesBencoding>),
}

pub fn to_string(to_parse: String) -> ResultBencoding<TupleStringRest> {
    let mut result = String::new();
    let mut long_string = String::new();
    let mut valid_format = false;

    //Tomo todos los valores antes del ':' que deberian representar el largo del string
    let mut list_chars = to_parse.chars();
    for long_char in list_chars.by_ref() {
        if long_char == ':' {
            valid_format = true;
            break;
        }
        long_string.push(long_char);
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

pub fn from_string(to_bencode: String) -> String {
    let mut bencoding = String::new();
    let long_number = to_bencode.len() as u32;
    let long_str = long_number.to_string();

    bencoding.push_str(&long_str);
    bencoding.push(':');
    bencoding.push_str(&to_bencode);

    bencoding
}

fn is_valid_number(num: String) -> bool {
    if num == "-0" {
        return false;
    }
    let mut chars_num = num.chars();

    if let Some(digit) = chars_num.next() {
        if digit == '-' {
            return is_valid_number(chars_num.collect());
        } else if digit == '0' && chars_num.next().is_some() {
            return false;
        }
    }
    true
}

pub fn to_integer(to_parse: String) -> ResultBencoding<TupleIntegerRest> {
    let mut num_str = String::new();
    let mut valid_format = false;
    let mut list_chars = to_parse.chars();

    //Valido que el primer caracter sea 'i'
    if let Some('i') = list_chars.next() {
    } else {
        return Err(ErrorBencoding::Integer(ErrorType::Format));
    }

    for num_char in list_chars.by_ref() {
        if num_char == 'e' {
            valid_format = true;
            break;
        }
        num_str.push(num_char);
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

pub fn from_integer(to_bencode: i64) -> String {
    let mut bencoding = String::from("i");
    let num_string = to_bencode.to_string();
    bencoding.push_str(&num_string);
    bencoding.push('e');

    bencoding
}

fn take_value_by_type(
    from: char,
    type_char: char,
    to_parse: String,
) -> ResultBencoding<TupleValueRest> {
    if type_char.is_ascii_digit() {
        let (str, next_parse) = to_string(to_parse)?;
        Ok((ValuesBencoding::String(str), next_parse))
    } else if type_char == 'i' {
        let (int, next_parse) = to_integer(to_parse)?;
        Ok((ValuesBencoding::Integer(int), next_parse))
    } else if type_char == 'l' {
        let (list, next_parse) = to_list(to_parse)?;
        Ok((ValuesBencoding::List(list), next_parse))
    } else if type_char == 'd' {
        let (dic, next_parse) = to_dic(to_parse)?;
        Ok((ValuesBencoding::Dic(dic), next_parse))
    } else if from == 'l' {
        Err(ErrorBencoding::List(ErrorType::Format))
    } else {
        Err(ErrorBencoding::Dic(ErrorType::Format))
    }
}

pub fn to_list(to_parse: String) -> ResultBencoding<TupleListRest> {
    let mut list_return = Vec::new();
    let mut valid_format = false;
    let mut list_chars = to_parse.chars();

    //Reviso que el string comience con 'l'
    match list_chars.next() {
        Some('l') => (),
        _ => return Err(ErrorBencoding::List(ErrorType::Format)),
    }

    let mut to_parse: String = list_chars.clone().collect();

    while let Some(next_char) = list_chars.next() {
        if next_char == 'e' {
            valid_format = true;
            break;
        }
        let (value, next_parse) = take_value_by_type('l', next_char, to_parse)?;
        list_return.push(value);
        to_parse = next_parse;
        list_chars = to_parse.chars();
    }

    if !valid_format {
        return Err(ErrorBencoding::List(ErrorType::Format));
    }

    Ok((list_return, list_chars.collect()))
}

pub fn from_list(to_bencode: Vec<ValuesBencoding>) -> String {
    let mut bencoding = String::from("l");
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

    bencoding.push('e');
    bencoding
}

pub fn to_dic(to_parse: String) -> ResultBencoding<TupleDicRest> {
    let mut dic_return: HashMap<String, ValuesBencoding> = HashMap::new();
    let mut valid_format = false;
    let mut list_chars = to_parse.chars();

    //Reviso que el string comience con 'd'
    match list_chars.next() {
        Some('d') => (),
        _ => return Err(ErrorBencoding::Dic(ErrorType::Format)),
    }

    let mut to_parse: String = list_chars.clone().collect();

    while let Some(next_char) = list_chars.next() {
        if next_char == 'e' {
            valid_format = true;
            break;
        }
        let (key, next_parse) = match to_string(to_parse.clone()) {
            Ok((k, p)) => (k, p),
            Err(_) => return Err(ErrorBencoding::Dic(ErrorType::Format)),
        };
        if let Some(char_next) = next_parse.chars().next() {
            let (value, next_parse) = take_value_by_type('d', char_next, next_parse)?;
            dic_return.insert(key, value);
            to_parse = next_parse;
        } else {
            return Err(ErrorBencoding::Dic(ErrorType::Format));
        }

        list_chars = to_parse.chars();
    }

    if !valid_format {
        return Err(ErrorBencoding::Dic(ErrorType::Format));
    }

    Ok((dic_return, list_chars.collect()))
}

pub fn from_dic(to_bencode: HashMap<String, ValuesBencoding>) -> String {
    let mut bencoding = String::from("d");

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

    bencoding.push('e');
    bencoding
}

#[cfg(test)]
mod tests {
    use super::*;
    mod tests_to_strings {
        use super::*;
        #[test]
        fn to_string_ok() {
            let bencoding_string = String::from("3:exe");
            let return_str = String::from("exe");
            let return_rest = String::from("");

            let result = to_string(bencoding_string);
            assert!(result.is_ok());
            let (str, rest) = result.unwrap();
            assert_eq!(str, return_str);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_string_ok_rest_valid() {
            let bencoding_string = String::from("5:magic4:testi32e");
            let return_str = String::from("magic");
            let return_rest = String::from("4:testi32e");

            let result = to_string(bencoding_string);
            assert!(result.is_ok());

            let (str, rest) = result.unwrap();
            assert_eq!(str, return_str);
            assert_eq!(rest, return_rest);

            let return_str = String::from("test");
            let return_rest = String::from("i32e");

            let result = to_string(rest);
            assert!(result.is_ok());

            let (str, rest) = result.unwrap();
            assert_eq!(str, return_str);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_string_error_format() {
            let bencoding_string = String::from("4exe");
            assert_eq!(
                to_string(bencoding_string),
                Err(ErrorBencoding::String(ErrorType::Format))
            );
        }
        #[test]
        fn to_string_error_without_number() {
            let bencoding_string = String::from("test");
            assert_eq!(
                to_string(bencoding_string),
                Err(ErrorBencoding::String(ErrorType::Format))
            );
        }
        #[test]
        fn to_string_error_invalid_number() {
            let bencoding_string = String::from("a:test");
            assert_eq!(
                to_string(bencoding_string),
                Err(ErrorBencoding::String(ErrorType::Format))
            );
        }
        #[test]
        fn to_string_error_invalid_long() {
            let bencoding_string = String::from("12:test");
            assert_eq!(
                to_string(bencoding_string),
                Err(ErrorBencoding::String(ErrorType::Long))
            );
        }
    }
    mod tests_to_integers {
        use super::*;
        #[test]
        fn to_integer_ok_positive() {
            let bencoding_int = String::from("i32e");
            let return_int = 32;
            let return_rest = String::from("");

            let result = to_integer(bencoding_int);
            assert!(result.is_ok());

            let (int, rest) = result.unwrap();
            assert_eq!(int, return_int);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_integer_ok_negative() {
            let bencoding_int = String::from("i-320e");
            let return_int = -320;
            let return_rest = String::from("");

            let result = to_integer(bencoding_int);
            assert!(result.is_ok());

            let (int, rest) = result.unwrap();
            assert_eq!(int, return_int);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_integer_ok_rest_valid() {
            let bencoding_int = String::from("i32ei-200e4:test");
            let return_int = 32;
            let return_rest = String::from("i-200e4:test");

            let result = to_integer(bencoding_int);
            assert!(result.is_ok());

            let (str, rest) = result.unwrap();
            assert_eq!(str, return_int);
            assert_eq!(rest, return_rest);

            let return_int = -200;
            let return_rest = String::from("4:test");

            let result = to_integer(rest);
            assert!(result.is_ok());

            let (str, rest) = result.unwrap();
            assert_eq!(str, return_int);
            assert_eq!(rest, return_rest);
        }
        #[test]
        fn to_integer_error_format() {
            let bencoding_int = String::from("32e");
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Format))
            );

            let bencoding_int = String::from("i32");
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Format))
            );
        }
        #[test]
        fn to_integer_error_minus_zero() {
            let bencoding_int = String::from("i-0e");
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );
        }
        #[test]
        fn to_integer_error_zero_and_number() {
            let bencoding_int = String::from("i018e");
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );

            let bencoding_int = String::from("i-08e");
            assert_eq!(
                to_integer(bencoding_int),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );
        }
        #[test]
        fn to_integer_error_invalid_number() {
            let bencoding_int = String::from("i2a3e");
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
            let str_expected = ValuesBencoding::String(String::from("test"));
            let int_expected = ValuesBencoding::Integer(32);
            let rest_expected = String::from("3:exe");
            let result_expected = (vec![str_expected, int_expected], rest_expected);
            let to_parse = String::from("l4:testi32ee3:exe");
            let result = to_list(to_parse);
            assert_eq!(result, Ok(result_expected));
        }
        #[test]
        fn to_list_inside_list_ok() {
            let str_expected = ValuesBencoding::String(String::from("test"));
            let int_expected = ValuesBencoding::Integer(32);
            let vec_expected = ValuesBencoding::List(vec![str_expected, int_expected]);
            let rest_expected = String::from("3:exe");
            let result_expected = (vec![vec_expected], rest_expected);
            let to_parse = String::from("ll4:testi32eee3:exe");
            let result = to_list(to_parse);
            assert_eq!(result, Ok(result_expected));
        }
        #[test]
        fn to_list_error_format() {
            let to_parse = String::from("4:testi32ee3:exe");
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::List(ErrorType::Format))
            );

            let to_parse = String::from("la:testi32ee3:exe");
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::List(ErrorType::Format))
            );
        }
        #[test]
        fn to_list_error_not_close() {
            let to_parse = String::from("l4:testi32e3:exe");
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::List(ErrorType::Format))
            );
        }
        #[test]
        fn to_list_error_string() {
            let to_parse = String::from("l4teste");
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::String(ErrorType::Format))
            );

            let to_parse = String::from("l10:teste");
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::String(ErrorType::Long))
            );
        }
        #[test]
        fn to_list_error_integer() {
            let to_parse = String::from("li-0ee");
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );

            let to_parse = String::from("li032ee");
            assert_eq!(
                to_list(to_parse),
                Err(ErrorBencoding::Integer(ErrorType::Number))
            );

            let to_parse = String::from("li5");
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
            let bencoding = String::from("d8:announcei32e4:test3:exee3:exe");
            let mut dic_expected = HashMap::new();
            dic_expected.insert("announce".to_owned(), ValuesBencoding::Integer(32));
            dic_expected.insert("test".to_owned(), ValuesBencoding::String("exe".to_owned()));
            let rest_expected = String::from("3:exe");

            assert_eq!(Ok((dic_expected, rest_expected)), to_dic(bencoding));
        }
        #[test]
        fn to_dic_invalid_format() {
            let bencoding = String::from("8:announcei32e4:test3:exee3:exe");
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = String::from("d8:announcei32e4:test3:exe3:exe");
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = String::from("d8:announcei32e4:test3:exe");
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );
        }
        #[test]
        fn to_dic_invalid_key() {
            let bencoding = String::from("di0ei32e4:test3:exee3:exe");
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );
        }
        #[test]
        fn to_dic_invalid_num() {
            let bencoding = String::from("d8:announcei-0e4:test3:exee3:exe");
            assert_eq!(
                Err(ErrorBencoding::Integer(ErrorType::Number)),
                to_dic(bencoding)
            );
        }
        #[test]
        fn to_dic_invalid_list() {
            let bencoding = String::from("d8:announcei32e4:testl2:el");
            assert_eq!(
                Err(ErrorBencoding::List(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = String::from("d8:announcei32e4:testlf:ele");
            assert_eq!(
                Err(ErrorBencoding::List(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = String::from("d8:announcei32e4:testl2:eli-0eee");
            assert_eq!(
                Err(ErrorBencoding::Integer(ErrorType::Number)),
                to_dic(bencoding)
            );
        }
        #[test]
        fn to_dic_invalid_dic() {
            let bencoding = String::from("d8:announcei32e4:testdi32ee");
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = String::from("d8:announcei32e4:testd3:exei-12e");
            assert_eq!(
                Err(ErrorBencoding::Dic(ErrorType::Format)),
                to_dic(bencoding)
            );

            let bencoding = String::from("d8:announcei32e3:inid4:testi-0ee");
            assert_eq!(
                Err(ErrorBencoding::Integer(ErrorType::Number)),
                to_dic(bencoding)
            );
        }
    }
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
            let rest = String::from("");
            dic.insert("A".to_owned(), ValuesBencoding::String("Meta".to_owned()));
            dic.insert("B".to_owned(), ValuesBencoding::Integer(-125));
            dic.insert("C".to_owned(), ValuesBencoding::Integer(0));
            dic.insert("D".to_owned(), ValuesBencoding::String("Fin".to_owned()));

            let bencoding = from_dic(dic.clone());

            assert_eq!(to_dic(bencoding), Ok((dic, rest)));
        }
    }
}
