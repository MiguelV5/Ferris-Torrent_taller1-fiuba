#![allow(dead_code)]
const MAX_CHAR_VAL: u32 = std::char::MAX as u32;

pub fn url_encode(to_encode: String) -> String {
    let mut buff = [0; 4]; // se usa para encode_utf8(), donde dice que un buffer de tamaño 4 es suficiente para encodear cualquier char
    let encoded = to_encode
        .chars()
        .map(|ch| match ch as u32 {
            //Son los chars que se deben codificar con formato %xx
            0..=47 | 58..=64 | 91..=96 | 123..=MAX_CHAR_VAL => {
                ch.encode_utf8(&mut buff);
                buff[0..ch.len_utf8()]
                    .iter()
                    .map(|&byte| format!("%{:02x}", byte))
                    .collect::<String>()
            }
            _ => ch.to_string(),
        })
        .collect::<String>();
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn space_encodes_to_percent20_ok() {
        let space = String::from(" ");
        let result = url_encode(space);
        assert_eq!(result, String::from("%20"));
    }
    #[test]
    fn ascii_control_characters_encodes_ok() {
        let bytes = &[0x00u8, 0x0cu8, 0x08u8, 0x1eu8, 0x09u8];
        let to_encode = String::from_utf8_lossy(bytes).to_string();
        let result = url_encode(to_encode);
        assert_eq!(result, String::from("%00%0c%08%1e%09"));
    }

    #[test]
    fn ascii_chars_encodes_ok() {
        let to_encode = String::from("abcdefABCDEF");
        let result = url_encode(to_encode);
        assert_eq!(result, String::from("abcdefABCDEF"));
    }

    #[test]
    fn special_ascii_chars_encodes_ok() {
        let to_encode = String::from("$&+,/:;=?@");
        let result = url_encode(to_encode);
        assert_eq!(result, String::from("%24%26%2b%2c%2f%3a%3b%3d%3f%40"));
    }

    #[test]
    fn unsafe_ascii_chars_encodes_ok() {
        let to_encode = String::from(" <>#%{}|^~[]`");
        let result = url_encode(to_encode);
        assert_eq!(
            result,
            String::from("%20%3c%3e%23%25%7b%7d%7c%5e%7e%5b%5d%60")
        );
    }
}
