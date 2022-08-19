//! # Modulo de URLdecoding
//! Modulo que contiene la función principal para realizar el decoding de Percent-encoding a partir de los bytes correspondientes.

/// Se encarga de decodificar desde formato urlencoding a bytes.
/// Recibe los bytes correspondientes a un String con percent-encoding y lo devuelve decodificado (tambien en bytes) según sus caracteres ascii
///
pub fn from_url(url: Vec<u8>) -> Vec<u8> {
    //Este nombre es para seguir la convencion que teniamos en el tp viejo
    let mut counter = 0;
    let mut hex = String::new();
    let mut vec_res = vec![];
    for byte in url {
        if byte == b'%' {
            counter = 2;
            continue;
        } else if counter > 0 {
            hex.push(byte as char);
            counter -= 1;
            if counter == 0 {
                if let Ok(num) = u8::from_str_radix(&hex, 16) {
                    vec_res.push(num)
                };
                hex = String::new();
            };
        } else {
            vec_res.push(byte);
        }
    }
    vec_res
}

#[cfg(test)]
mod tests_urlencoding_decoder {
    use super::*;

    #[test]
    fn percent20_decodes_to_space_ok() {
        let encoded_space = String::from("%20").as_bytes().to_vec();
        let result = from_url(encoded_space);
        let expected_result = " ".as_bytes().to_vec();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn decodes_to_ascii_control_characters_ok() {
        let encoded_control_characters = String::from("%00%0c%08%1e%09").as_bytes().to_vec();
        let result = from_url(encoded_control_characters);
        let hex_bytes = &[0x00u8, 0x0cu8, 0x08u8, 0x1eu8, 0x09u8];
        let expected_result = String::from_utf8_lossy(hex_bytes)
            .to_string()
            .as_bytes()
            .to_vec();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn decodes_to_normal_ascii_chars_only_ok() {
        let encoded_chars = String::from("abcdefABCDEF").as_bytes().to_vec();
        let result = from_url(encoded_chars);
        let expected_result = "abcdefABCDEF".as_bytes().to_vec();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn decodes_to_special_ascii_chars_only_ok() {
        let encoded_special_chars = String::from("%24%26%2b%2c%2f%3a%3b%3d%3f%40")
            .as_bytes()
            .to_vec();
        let result = from_url(encoded_special_chars);
        let expected_result = "$&+,/:;=?@".as_bytes().to_vec();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn decodes_to_unsafe_ascii_chars_ok() {
        let encoded_unsafe_chars = String::from("%20%3c%3e%23%25%7b%7d%7c%5e%5b%5d%60")
            .as_bytes()
            .to_vec();
        let result = from_url(encoded_unsafe_chars);
        let expected_result = " <>#%{}|^[]`".as_bytes().to_vec();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn decodes_to_mixed_ascii_chars_ok() {
        let encoded_mixed_chars = String::from("%20A%3c%3ed%23%25%7b%7d%7c%5e~%5b%5dRR%60mpqZ")
            .as_bytes()
            .to_vec();
        let result = from_url(encoded_mixed_chars);
        let expected_result = " A<>d#%{}|^~[]RR`mpqZ".as_bytes().to_vec();
        assert_eq!(result, expected_result);
    }

    #[test]
    fn decodes_to_direct_hex_bytes_ok() {
        let encoded_bytes = String::from("%ac%c3%b2%e43%d7%c7GZ%bbYA%b5h%1c%b7%a1%ea%26%e2")
            .as_bytes()
            .to_vec();
        let result = from_url(encoded_bytes);

        let hex_bytes = &[
            0xACu8, 0xC3u8, 0xB2u8, 0xE4u8, 0x33u8, 0xD7u8, 0xC7u8, 0x47u8, 0x5Au8, 0xBBu8, 0x59u8,
            0x41u8, 0xB5u8, 0x68u8, 0x1Cu8, 0xB7u8, 0xA1u8, 0xEAu8, 0x26u8, 0xE2u8,
        ];
        let expected_result = hex_bytes.to_vec();
        assert_eq!(result, expected_result);
    }
}
