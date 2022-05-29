/// Se encarga de codificar en formato urlencoding.
/// Recibe un String y lo devuelve codificado según sus caracteres ascii
///
/// Ejemplo
/// ```
/// # use fa_torrent::torrent::parsers::url_encoder;
/// let to_encode = " A<>d#%{}|^~[]RR`mpqZ".as_bytes().to_vec();
/// let result = url_encoder::from_string(to_encode);
/// ```
pub fn from_string(to_encode: Vec<u8>) -> Vec<u8> {
    to_encode
        .into_iter()
        .map(|ch| match ch {
            //Son los chars que no se deben codificar con formato %xx
            45 | 46 | 48..=57 | 65..=90 | 95 | 97..=122 | 126 => (ch as char).to_string(),
            _ => format!("%{:02x}", ch),
        })
        .collect::<String>()
        .as_bytes()
        .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn space_encodes_to_percent20_ok() {
        let space = String::from(" ").as_bytes().to_vec();
        let result = from_string(space);
        assert_eq!(result, "%20".as_bytes().to_vec());
    }
    #[test]
    fn ascii_control_characters_encodes_ok() {
        let bytes = &[0x00u8, 0x0cu8, 0x08u8, 0x1eu8, 0x09u8];
        let to_encode = String::from_utf8_lossy(bytes)
            .to_string()
            .as_bytes()
            .to_vec();
        let result = from_string(to_encode);
        assert_eq!(result, "%00%0c%08%1e%09".as_bytes().to_vec());
    }

    #[test]
    fn ascii_chars_encodes_ok() {
        let to_encode = "abcdefABCDEF".as_bytes().to_vec();
        let result = from_string(to_encode);
        assert_eq!(result, "abcdefABCDEF".as_bytes().to_vec());
    }

    #[test]
    fn special_ascii_chars_encodes_ok() {
        let to_encode = "$&+,/:;=?@".as_bytes().to_vec();
        let result = from_string(to_encode);
        assert_eq!(result, "%24%26%2b%2c%2f%3a%3b%3d%3f%40".as_bytes().to_vec());
    }

    #[test]
    fn unsafe_ascii_chars_encodes_ok() {
        let to_encode = " <>#%{}|^[]`".as_bytes().to_vec();
        let result = from_string(to_encode);
        assert_eq!(
            result,
            "%20%3c%3e%23%25%7b%7d%7c%5e%5b%5d%60".as_bytes().to_vec()
        );
    }

    #[test]
    fn mix_ascii_chars_encodes_ok() {
        let to_encode = " A<>d#%{}|^~[]RR`mpqZ".as_bytes().to_vec();
        let result = from_string(to_encode);
        assert_eq!(
            result,
            "%20A%3c%3ed%23%25%7b%7d%7c%5e~%5b%5dRR%60mpqZ"
                .as_bytes()
                .to_vec()
        );
    }
}
