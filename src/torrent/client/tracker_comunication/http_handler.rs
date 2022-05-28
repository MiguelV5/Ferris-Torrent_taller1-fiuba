#![allow(dead_code)]
extern crate sha1;

use native_tls::{TlsConnector, TlsStream};

use crate::torrent::data::torrent_file_data::TorrentFileData;
use crate::torrent::parsers::bencoding::decoder::to_dic;
use crate::torrent::parsers::bencoding::values::ValuesBencoding;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;

type DicValues = HashMap<Vec<u8>, ValuesBencoding>;
type ResultMsg<T> = Result<T, ErrorMsgHttp>;

const ONE: usize = 1;
const TWO: usize = 2;
const FOUR: usize = 4;

const LAST_SLASH: &[u8; ONE] = b"/";
const TWO_SLASH: &[u8; TWO] = b"//";
const TWO_POINTS: &[u8; ONE] = b":";
const END_LINE: &[u8; TWO] = b"\r\n";
const DOUBLE_END_LINE: &[u8; FOUR] = b"\r\n\r\n";

const INIT_MSG: &str = "GET /announce";
const INFO: &str = "info";
const INFO_HASH: &str = "?info_hash=";
const PEER_ID: &str = "&peer_id=";
const IP: &str = "&ip=";
const PORT: &str = "&port=";
const UPLOADED: &str = "&uploaded=";
const DOWNLOADED: &str = "&downloaded=";
const LEFT: &str = "&left=";
const EVENT: &str = "&event=";
const HTTP: &str = " HTTP/1.0\r\n";
const HOST: &str = "Host:";
const MSG_ENDING: &str = "\r\n\r\n";

#[derive(Debug, PartialEq)]
pub enum ErrorMsgHttp {
    NoAnnounce,
    NoInfoHash,
    ToDicError,
    FormatResponseError,
    CreateTls,
    ConnectTcp,
    ConnectTls,
    NoConection,
    HttpDescription(String),
    HttpCode(u32),
    SendingGetMessage,
    ReadingResponse,
}

struct MsgDescriptor {
    info_hash: String,
    peer_id: String,
    ip: String,
    port: String,
    uploaded: String,
    downloaded: String,
    left: String,
    event: String,
    host: String,
}

fn vec_u8_to_string(vec: &[u8]) -> String {
    String::from_utf8_lossy(vec).to_string()
}

fn find_index_msg(response: &[u8], size: usize, end_line: &[u8]) -> Option<usize> {
    response.windows(size).position(|arr| arr == end_line)
}

//Paso url del tracker al formato que necesito de host.
//Ej: pasaria de http://torrent.ubuntu.com:6969/announce a torrent.ubuntu.com
fn init_host(tracker: String) -> ResultMsg<String> {
    let u8_tracker = tracker.as_bytes();
    //Voy a quitar todo lo que este por detras del "//"
    match find_index_msg(u8_tracker, TWO, TWO_SLASH) {
        Some(pos) => {
            let first = pos + TWO;
            //Voy a quitar todo lo que este por delante del ":"
            match find_index_msg(&u8_tracker[first..], ONE, TWO_POINTS) {
                Some(pos) => {
                    let last = pos + first;
                    Ok(vec_u8_to_string(&u8_tracker[first..last].to_vec()))
                }
                //Si no hay ":" quito todo lo que haya por delante del "/"
                None => match find_index_msg(&u8_tracker[first..], ONE, LAST_SLASH) {
                    Some(pos) => {
                        let last = pos + first;
                        Ok(vec_u8_to_string(&u8_tracker[first..last].to_vec()))
                    }
                    None => Err(ErrorMsgHttp::NoAnnounce),
                },
            }
        }
        None => Err(ErrorMsgHttp::NoAnnounce),
    }
}

impl MsgDescriptor {
    pub fn new(torrent: TorrentFileData) -> ResultMsg<Self> {
        let info_hash = torrent.info_hash;
        let peer_id = String::from("ABCDEFGHIJKLMNOPQRST"); //Crear generador de Peer_ID
        let ip = String::from("127.0.0.1");
        let port = String::from("6881"); //Puertos que se utilizan (6881-6889)
        let uploaded = String::from("0");
        let downloaded = String::from("0");
        let left = torrent.total_size.to_string();
        let event = String::from("started");
        let host = init_host(torrent.tracker_main)?;

        Ok(MsgDescriptor {
            info_hash,
            peer_id,
            ip,
            port,
            uploaded,
            downloaded,
            left,
            event,
            host,
        })
    }
    pub fn get_info_hash(&self) -> String {
        self.info_hash.clone()
    }
    pub fn get_peer_id(&self) -> String {
        self.peer_id.clone()
    }
    pub fn get_ip(&self) -> String {
        self.ip.clone()
    }
    pub fn get_port(&self) -> String {
        self.port.clone()
    }
    pub fn get_uploaded(&self) -> String {
        self.uploaded.clone()
    }
    pub fn get_downloaded(&self) -> String {
        self.downloaded.clone()
    }
    pub fn get_left(&self) -> String {
        self.left.clone()
    }
    pub fn get_event(&self) -> String {
        self.event.clone()
    }
    pub fn get_host(&self) -> String {
        self.host.clone()
    }
}

pub struct HttpHandler {
    msg_get: MsgDescriptor,
}

fn add_description_msg(msg: &mut String, type_msg: &str, value: String) {
    msg.push_str(type_msg);
    msg.push_str(&value);
}

impl HttpHandler {
    pub fn new(torrent: TorrentFileData) -> ResultMsg<Self> {
        Ok(HttpHandler {
            msg_get: MsgDescriptor::new(torrent)?,
        })
    }

    pub fn get_host(&self) -> String {
        self.msg_get.get_host()
    }

    pub fn get_send_msg(&self) -> ResultMsg<String> {
        let mut result = String::from(INIT_MSG);
        add_description_msg(&mut result, INFO_HASH, self.msg_get.get_info_hash());
        add_description_msg(&mut result, PEER_ID, self.msg_get.get_peer_id());
        add_description_msg(&mut result, IP, self.msg_get.get_ip());
        add_description_msg(&mut result, PORT, self.msg_get.get_port());
        add_description_msg(&mut result, UPLOADED, self.msg_get.get_uploaded());
        add_description_msg(&mut result, DOWNLOADED, self.msg_get.get_downloaded());
        add_description_msg(&mut result, LEFT, self.msg_get.get_left());
        add_description_msg(&mut result, EVENT, self.msg_get.get_event());
        add_description_msg(&mut result, HTTP, String::new());
        add_description_msg(&mut result, HOST, self.msg_get.get_host());
        add_description_msg(&mut result, MSG_ENDING, String::new());
        Ok(result)
    }

    pub fn connect(&self) -> ResultMsg<TlsStream<TcpStream>> {
        let connector = match TlsConnector::new() {
            Ok(conected) => conected,
            Err(_) => return Err(ErrorMsgHttp::CreateTls),
        };

        let mut addr = self.get_host();
        addr.push_str(":443");

        let stream = match TcpStream::connect(addr) {
            Ok(tcp_conected) => tcp_conected,
            Err(_) => return Err(ErrorMsgHttp::ConnectTcp),
        };

        let domain = self.get_host();
        let connection = match connector.connect(&domain, stream) {
            Ok(tls_conected) => tls_conected,
            Err(_) => return Err(ErrorMsgHttp::ConnectTcp),
        };
        Ok(connection)
    }

    fn check_http_code(&self, response: &[u8]) -> ResultMsg<()> {
        let string_response = String::from_utf8_lossy(response).to_string();
        let mut iter_string = string_response.split(' ');
        match iter_string.next() {
            Some("HTTP/1.1") => (),
            _ => return Err(ErrorMsgHttp::FormatResponseError),
        }
        match iter_string.next() {
            Some("200") => (),
            Some(code) => match code.parse::<u32>() {
                Ok(number_code) => return Err(ErrorMsgHttp::HttpCode(number_code)),
                _ => return Err(ErrorMsgHttp::FormatResponseError),
            },
            _ => return Err(ErrorMsgHttp::FormatResponseError),
        }
        match iter_string.next() {
            Some("OK") => (),
            Some(msg_error) => {
                let mut message = msg_error.to_owned();
                for more_msg in iter_string {
                    message.push(' ');
                    message.push_str(more_msg);
                }
                return Err(ErrorMsgHttp::HttpDescription(message));
            }
            _ => return Err(ErrorMsgHttp::FormatResponseError),
        }
        Ok(())
    }

    fn tracker_response_to_dic(&self, response: Vec<u8>) -> ResultMsg<DicValues> {
        //Reviso que la primer linea de la respuesta me de una respuesta y codigo valido.
        match find_index_msg(&response, TWO, END_LINE) {
            Some(pos) => self.check_http_code(&response[..pos])?,
            None => return Err(ErrorMsgHttp::FormatResponseError),
        }
        //Tomo el diccionario en bencoding de la respuesta y lo paso a HashMap
        match find_index_msg(&response, FOUR, DOUBLE_END_LINE) {
            Some(pos) => {
                let bencode_response = response[(pos + FOUR)..].to_vec();
                if let Ok((dic_response, _)) = to_dic(bencode_response) {
                    Ok(dic_response)
                } else {
                    Err(ErrorMsgHttp::ToDicError)
                }
            }
            None => Err(ErrorMsgHttp::FormatResponseError),
        }
    }

    pub fn tracker_get_response(&self) -> ResultMsg<DicValues> {
        let mut connector = self.connect()?;

        let get_msg = self.get_send_msg()?;
        if connector.write_all(get_msg.as_bytes()).is_err() {
            return Err(ErrorMsgHttp::SendingGetMessage);
        };

        let mut response_tracker = vec![];
        if connector.read_to_end(&mut response_tracker).is_err() {
            return Err(ErrorMsgHttp::ReadingResponse);
        }

        self.tracker_response_to_dic(response_tracker)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::torrent::data::medatada_analyzer::read_torrent_file_to_dic;
    #[test]
    fn test_creation_file1_ok() {
        //ubuntu-22.04-desktop-amd64.iso
        //big-buck-bunny
        //ubuntu-14.04.6-server-ppc64el.iso
        let dir = "torrents_for_test/ubuntu-22.04-desktop-amd64.iso.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => panic!("{:?}", error),
        };

        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => panic!("{:?}", error),
        };

        let http_handler = match HttpHandler::new(torrent.clone()) {
            Ok(handler) => handler,
            Err(error) => panic!("{:?}", error),
        };

        let mut msg_get_expected = String::from("GET /announce");
        msg_get_expected.push_str("?info_hash=");
        msg_get_expected.push_str(&torrent.get_info_hash());
        msg_get_expected.push_str("&peer_id=ABCDEFGHIJKLMNOPQRST&ip=127.0.0.1&port=6881");
        msg_get_expected.push_str("&uploaded=0&downloaded=0&left=");
        msg_get_expected.push_str(&torrent.get_total_size().to_string());
        msg_get_expected.push_str("&event=started HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n");

        assert_eq!(http_handler.get_send_msg(), Ok(msg_get_expected))
    }
}