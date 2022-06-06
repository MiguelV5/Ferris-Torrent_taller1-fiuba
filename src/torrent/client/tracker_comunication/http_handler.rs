#![allow(dead_code)]
extern crate sha1;

use native_tls::{TlsConnector, TlsStream};

use crate::torrent::data::torrent_file_data::TorrentFileData;
use crate::torrent::parsers::bencoding;
use crate::torrent::parsers::bencoding::values::ValuesBencoding;
use crate::torrent::parsers::url_encoder;
use log::{debug, error, trace};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
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
//const COMPACT: &str = "&compact=";
const LEFT: &str = "&left=";
const EVENT: &str = "&event=";
const HTTP: &str = " HTTP/1.0\r\n";
const HOST: &str = "Host:";
const MSG_ENDING: &str = "\r\n\r\n";

const IP_CLIENT: &str = "127.0.0.1";
const PORT_CLIENT: &str = ":443";
const STARTED: &str = "started";
const INIT_PORT: u32 = 6881;

///Enumerado que representa los tipos de error que pueden surgir
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
    NoMorePorts,
    HttpDescription(String),
    SendingGetMessage,
    ReadingResponse,
}

impl fmt::Display for ErrorMsgHttp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error del torrent.\n Backtrace: {:?}\n", self)
    }
}

impl Error for ErrorMsgHttp {}

struct MsgDescriptor {
    info_hash: String,
    peer_id: String,
    ip: String,
    port: u32,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    //[POR HACER: Ver como hacer para recibir respuestas compactas]
    //compact: u8,
    event: String,
    host: String,
}

fn vec_u8_to_string(vec: &[u8]) -> String {
    String::from_utf8_lossy(vec).into_owned()
}

fn find_index_msg(response: &[u8], size: usize, end_line: &[u8]) -> Option<usize> {
    response.windows(size).position(|arr| arr == end_line)
}

fn init_info_hash(vec_sha1: Vec<u8>) -> String {
    let info_hash = url_encoder::from_string(vec_sha1);
    vec_u8_to_string(&info_hash)
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

fn add_description_msg(msg: &mut String, type_msg: &str, value: String) {
    msg.push_str(type_msg);
    msg.push_str(&value);
}

impl MsgDescriptor {
    ///Funcion que va a crear un MsgDescriptor, la cual necesita para crearse un TorrentFileData y
    /// un peer_id, esta estructura va a servir para crear el mensaje de request al tracker
    pub fn new(torrent: TorrentFileData, peer_id: String) -> ResultMsg<Self> {
        let info_hash = init_info_hash(torrent.get_info_hash());
        let ip = String::from(IP_CLIENT);
        let port = INIT_PORT;
        let uploaded = 0;
        let downloaded = 0;
        let left = torrent.get_total_size() as u64;
        let event = String::from(STARTED);
        let host = init_host(torrent.get_tracker_main())?;
        //let compact = 0;

        Ok(MsgDescriptor {
            info_hash,
            peer_id,
            ip,
            port,
            //compact,
            uploaded,
            downloaded,
            left,
            event,
            host,
        })
    }
    ///Esta funcion devuelve el info_hash [ver [TorrentFileData]] url encodeado
    pub fn get_info_hash(&self) -> String {
        self.info_hash.clone()
    }
    ///Esta funcion devuelve el peer_id
    pub fn get_peer_id(&self) -> String {
        self.peer_id.clone()
    }
    ///Esta funcion devuelve la ip
    pub fn get_ip(&self) -> String {
        self.ip.clone()
    }
    ///Esta funcion devuelve el puerto
    pub fn get_port(&self) -> String {
        self.port.to_string()
    }
    ///Esta funcion devuelve la cantidad en bytes subidas del archivo
    /// en formato String
    pub fn get_uploaded(&self) -> String {
        self.uploaded.to_string()
    }
    ///Esta funcion devuelve la cantidad bajada en bytes del archivo
    /// en formato String
    pub fn get_downloaded(&self) -> String {
        self.downloaded.to_string()
    }

    //pub fn get_compact(&self) -> String {
    //    self.compact.to_string()
    //}
    ///Esta funcion devuelve la cantidad que falta descargar del archivo
    /// en bytes en formato String
    pub fn get_left(&self) -> String {
        self.left.to_string()
    }
    ///Esta funcion devuelve el event
    pub fn get_event(&self) -> String {
        self.event.clone()
    }
    ///Esta funcion devuelve el host
    pub fn get_host(&self) -> String {
        self.host.clone()
    }
    //[POR HACER: Realizar cambio de puertos en caso de que el primero falle]

    ///Esta funcion cambia el puerto para la comunicacion con el tracker, los puertos
    /// validos son del 6881 al 6889 si el cambio es exitoso devolvera Ok(()), en caso de llegar al
    /// ultimo puerto valido y querer volver a cambiar se devolvera un error y el puerto volvera a setearse
    /// al primer puerto valido.
    pub fn change_port(&mut self) -> Result<(), ErrorMsgHttp> {
        if self.port == 6889 {
            self.port = 6881;
            Err(ErrorMsgHttp::NoMorePorts)
        } else {
            self.port += 1;
            Ok(())
        }
    }
    ///Funcion que actualiza los valores de downloaded, uploaded y left
    pub fn update_download_stats(&mut self, more_down: u64, more_up: u64) {
        self.downloaded += more_down;
        self.uploaded += more_up;
        self.left -= more_down;
    }
    ///Funcion que devuelve el mensaje que debera ser enviado al tracker
    pub fn get_send_msg(&self) -> ResultMsg<String> {
        let mut result = String::from(INIT_MSG);
        add_description_msg(&mut result, INFO_HASH, self.get_info_hash());
        add_description_msg(&mut result, PEER_ID, self.get_peer_id());
        add_description_msg(&mut result, IP, self.get_ip());
        //add_description_msg(&mut result, COMPACT, self.get_compact());
        add_description_msg(&mut result, PORT, self.get_port());
        add_description_msg(&mut result, UPLOADED, self.get_uploaded());
        add_description_msg(&mut result, DOWNLOADED, self.get_downloaded());
        add_description_msg(&mut result, LEFT, self.get_left());
        add_description_msg(&mut result, EVENT, self.get_event());
        add_description_msg(&mut result, HTTP, String::new());
        add_description_msg(&mut result, HOST, self.get_host());
        add_description_msg(&mut result, MSG_ENDING, String::new());
        Ok(result)
    }
}

pub struct HttpHandler {
    msg_get: MsgDescriptor,
}

impl HttpHandler {
    ///Esta funcion creara el HttpHandler el cual es el encargado de comunicarse con el tracker,
    /// ya sea enviandole la request y recibiendo su respuesta y devolviendola en el HashMap correspondiente,
    /// esta estructura contiene una estructura MsgDescriptor que va a ser la que creara el request con el tracker,
    /// Para crear el HttpHandler necesitamos pasarle el TorrentFileData correspondiente al .torrent y un peer_id
    pub fn new(torrent: TorrentFileData, peer_id: String) -> ResultMsg<Self> {
        Ok(HttpHandler {
            msg_get: MsgDescriptor::new(torrent, peer_id)?,
        })
    }
    ///Devuelve el host del mensaje al tracker
    pub fn get_host(&self) -> String {
        self.msg_get.get_host()
    }
    ///Devuelve el mensaje de request que debe ser enviado al tracker
    pub fn get_send_msg(&self) -> ResultMsg<String> {
        self.msg_get.get_send_msg()
    }
    ///Funcion que actualiza los estados de downloaded, uploaded y left del MsgDescriptor almacenado
    pub fn update_download_stats(&mut self, more_down: u64, more_up: u64) {
        self.msg_get.update_download_stats(more_down, more_up)
    }
    ///Funcion que sirve para conectarse con el tracker correspondiente, en caso de que alguna de las conexiones
    /// falle se devolvera el error correspondiente
    pub fn connect(&self) -> ResultMsg<TlsStream<TcpStream>> {
        let connector = match TlsConnector::new() {
            Ok(conected) => conected,
            Err(_) => return Err(ErrorMsgHttp::CreateTls),
        };

        let mut addr = self.get_host();
        addr.push_str(PORT_CLIENT);
        debug!("Conectando TCP con addr: {}", addr);

        let stream = match TcpStream::connect(addr) {
            Ok(tcp_conected) => tcp_conected,
            Err(_) => {
                error!("Error al comunicarse con Tcp");
                return Err(ErrorMsgHttp::ConnectTcp);
            }
        };

        let domain = self.get_host();
        debug!("Conectando TLS con domain: {}", domain);
        let connection = match connector.connect(&domain, stream) {
            Ok(tls_conected) => tls_conected,
            Err(_) => {
                error!("Error al comunicarse con Tls");
                return Err(ErrorMsgHttp::ConnectTls);
            }
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
        let str_code = match iter_string.next() {
            Some(code) => code.to_owned(),
            _ => return Err(ErrorMsgHttp::FormatResponseError),
        };

        match iter_string.next() {
            Some(msg_error) => {
                let mut message = str_code.clone();
                message.push_str(": ");
                message.push_str(msg_error);
                for more_msg in iter_string {
                    message.push(' ');
                    message.push_str(more_msg);
                }
                match str_code.parse::<u32>() {
                    Ok(200..=299) => Ok(()),
                    Ok(_) => Err(ErrorMsgHttp::HttpDescription(message)),
                    Err(_) => Err(ErrorMsgHttp::FormatResponseError),
                }
            }
            _ => Err(ErrorMsgHttp::FormatResponseError),
        }
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
                if let Ok(dic_response) = bencoding::decoder::from_torrent_to_dic(bencode_response)
                {
                    Ok(dic_response)
                } else {
                    Err(ErrorMsgHttp::ToDicError)
                }
            }
            None => Err(ErrorMsgHttp::FormatResponseError),
        }
    }
    ///Funcion en la que le pedimos al HttpHandler que se conecte con el tracker, le envie el request
    /// correspondiente y luego nos devuelva la respuesta en formato de HashMap.
    ///
    /// Posibles errores que puede devolver:
    ///
    ///
    /// -En caso de que la respuesta nos de un codigo de error se devolvera el mismo junto con su descripcion
    ///
    /// -En caso de que no pueda conectarse en TCP o TLS se devolvera el error correspondiente
    ///
    /// -En caso de que haya un error en el envio del request o recepcion de la respuesta se devolvera el error
    ///  correspondiente
    pub fn tracker_get_response(&self) -> ResultMsg<DicValues> {
        let mut connector = self.connect()?;

        let get_msg = self.get_send_msg()?;
        trace!("Enviando request al tracker");
        debug!("Request: [{:?}]", get_msg);
        if connector.write_all(get_msg.as_bytes()).is_err() {
            error!("Error al escribir request al Tracker");
            return Err(ErrorMsgHttp::SendingGetMessage);
        };

        let mut response_tracker = vec![];
        trace!("Recibiendo respuesta del tracker");
        if connector.read_to_end(&mut response_tracker).is_err() {
            error!("Error al leer la respuesta del Tracker");
            return Err(ErrorMsgHttp::ReadingResponse);
        }
        debug!(
            "Response: [{:?}]",
            String::from_utf8_lossy(&response_tracker.clone())
        );
        self.tracker_response_to_dic(response_tracker)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::torrent::data::medatada_analyzer::read_torrent_file_to_dic;

    #[test]
    fn test_creation_file1_ok() {
        let dir = "torrents_for_test/ubuntu-22.04-desktop-amd64.iso.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => panic!("{}", error),
        };

        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => panic!("{}", error),
        };

        let http_handler =
            match HttpHandler::new(torrent.clone(), "ABCDEFGHIJKLMNOPQRST".to_string()) {
                Ok(handler) => handler,
                Err(error) => panic!("{}", error),
            };
        let info_hash = init_info_hash(torrent.get_info_hash());

        let mut msg_get_expected = String::from("GET /announce");
        msg_get_expected.push_str("?info_hash=");
        msg_get_expected.push_str(&info_hash);
        msg_get_expected.push_str("&peer_id=ABCDEFGHIJKLMNOPQRST&ip=127.0.0.1&port=6881");
        msg_get_expected.push_str("&uploaded=0&downloaded=0&left=");
        msg_get_expected.push_str(&torrent.get_total_size().to_string());
        msg_get_expected.push_str("&event=started HTTP/1.0\r\nHost:torrent.ubuntu.com\r\n\r\n");

        assert_eq!(http_handler.get_send_msg(), Ok(msg_get_expected))
    }
    #[test]
    fn test_creation_file2_ok() {
        let dir = "torrents_for_test/big-buck-bunny.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => panic!("{}", error),
        };

        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => panic!("{}", error),
        };

        let http_handler =
            match HttpHandler::new(torrent.clone(), "ABCDEFGHIJKLMNOPQRST".to_string()) {
                Ok(handler) => handler,
                Err(error) => panic!("{}", error),
            };
        let info_hash = init_info_hash(torrent.get_info_hash());

        let mut msg_get_expected = String::from("GET /announce");
        msg_get_expected.push_str("?info_hash=");
        msg_get_expected.push_str(&info_hash);
        msg_get_expected.push_str("&peer_id=ABCDEFGHIJKLMNOPQRST");
        msg_get_expected.push_str("&ip=");
        msg_get_expected.push_str(http_handler.msg_get.get_ip().as_str());
        msg_get_expected.push_str("&port=");
        msg_get_expected.push_str(http_handler.msg_get.get_port().as_str());
        msg_get_expected.push_str("&uploaded=0&downloaded=0&left=");
        msg_get_expected.push_str(&torrent.get_total_size().to_string());
        msg_get_expected.push_str("&event=started HTTP/1.0\r\nHost:");
        msg_get_expected.push_str(http_handler.msg_get.get_host().as_str());
        msg_get_expected.push_str("\r\n\r\n");

        assert_eq!(http_handler.get_send_msg(), Ok(msg_get_expected))
    }
    #[test]
    fn test_creation_file3_ok() {
        let dir = "torrents_for_test/ubuntu-14.04.6-server-ppc64el.iso.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => panic!("{}", error),
        };

        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => panic!("{}", error),
        };

        let http_handler =
            match HttpHandler::new(torrent.clone(), "ABCDEFGHIJKLMNOPQRST".to_string()) {
                Ok(handler) => handler,
                Err(error) => panic!("{}", error),
            };
        let info_hash = init_info_hash(torrent.get_info_hash());

        let mut msg_get_expected = String::from("GET /announce");
        msg_get_expected.push_str("?info_hash=");
        msg_get_expected.push_str(&info_hash);
        msg_get_expected.push_str("&peer_id=ABCDEFGHIJKLMNOPQRST");
        msg_get_expected.push_str("&ip=");
        msg_get_expected.push_str(http_handler.msg_get.get_ip().as_str());
        msg_get_expected.push_str("&port=");
        msg_get_expected.push_str(http_handler.msg_get.get_port().as_str());
        msg_get_expected.push_str("&uploaded=0&downloaded=0&left=");
        msg_get_expected.push_str(&torrent.get_total_size().to_string());
        msg_get_expected.push_str("&event=started HTTP/1.0\r\nHost:");
        msg_get_expected.push_str(http_handler.msg_get.get_host().as_str());
        msg_get_expected.push_str("\r\n\r\n");

        assert_eq!(http_handler.get_send_msg(), Ok(msg_get_expected))
    }
    #[test]
    fn test_check_http_code() {
        let dir = "torrents_for_test/ubuntu-22.04-desktop-amd64.iso.torrent";

        let dic_torrent = match read_torrent_file_to_dic(dir) {
            Ok(dic_torrent) => dic_torrent,
            Err(error) => panic!("{}", error),
        };

        let torrent = match TorrentFileData::new(dic_torrent) {
            Ok(struct_torrent) => struct_torrent,
            Err(error) => panic!("{}", error),
        };

        let http_handler =
            match HttpHandler::new(torrent.clone(), "ABCDEFGHIJKLMNOPQRST".to_string()) {
                Ok(handler) => handler,
                Err(error) => panic!("{}", error),
            };

        let response = http_handler.check_http_code("HTTP/1.1 200 OK".as_bytes());
        assert_eq!(response, Ok(()));

        let response = http_handler.check_http_code("HTTP/1.1 400 NOT FOUND".as_bytes());
        assert_eq!(
            response,
            Err(ErrorMsgHttp::HttpDescription("400: NOT FOUND".to_owned()))
        );
    }
}
