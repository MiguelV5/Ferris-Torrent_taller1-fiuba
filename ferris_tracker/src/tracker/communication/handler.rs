use std::{
    error::Error,
    fmt, fs,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use log::{error, info};

use crate::{
    tracker::{
        data::{
            constants::*,
            json::Json,
            peer_info::{get_error_response_for_announce, PeerInfo, PeerInfoError},
        },
        thread_pool::ThreadPool,
    },
    ArcMutexOfTorrents,
};

#[derive(Debug, Eq, PartialEq)]
pub enum CommunicationError {
    PoolExecutionError(String),
    ShutdownSettingError(String),
    ReadingFilesToFillResponseContentError(String),
    WritingResponse(String),
    UnlockingMutexOfTorrents,
    ReadingPeerSocket(String),
}

impl fmt::Display for CommunicationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for CommunicationError {}

// ========================================================================================

pub fn is_global_shutdown_set(global_shutdown: &Arc<RwLock<bool>>) -> bool {
    if let Ok(mutex_sutdown) = global_shutdown.read() {
        *mutex_sutdown
    } else {
        true // Si el global shutdown está poisoned, hay que cortar todo igual
    }
}

pub fn set_global_shutdown(global_shutdown: &Arc<RwLock<bool>>) -> Result<(), CommunicationError> {
    let mut global_shutdown = global_shutdown
        .write()
        .map_err(|err| CommunicationError::ShutdownSettingError(err.to_string()))?;
    *global_shutdown = true;
    Ok(())
}

fn get_response_details(
    buffer: &[u8],
    dic_torrents: &ArcMutexOfTorrents,
    json: &Arc<RwLock<Json>>,
    ip_port: SocketAddr,
) -> Vec<u8> {
    let info_of_announced_peer = PeerInfo::new((*buffer).to_vec(), ip_port);

    match info_of_announced_peer {
        Ok(info_of_announced_peer) => {
            let info_hash = info_of_announced_peer.get_info_hash();
            let peer_is_completed = info_of_announced_peer.is_complete();
            match dic_torrents.write() {
                Ok(mut unlocked_dic) => match unlocked_dic.get_mut(&info_hash) {
                    Some(torrent) => {
                        let response = torrent.get_bencoded_response_for_announce(
                            info_of_announced_peer.get_peer_id(),
                            info_of_announced_peer.is_compact(),
                        );
                        if torrent
                            .add_peer(info_of_announced_peer.get_peer_id(), info_of_announced_peer)
                        {
                            match json.write() {
                                Ok(mut unlocked_json) => {
                                    unlocked_json.add_new_connection(peer_is_completed);
                                }
                                Err(_) => {
                                    return get_error_response_for_announce(
                                        PeerInfoError::PoissonedLock,
                                    )
                                    .as_bytes()
                                    .to_vec();
                                }
                            };
                        };
                        response
                    }
                    None => get_error_response_for_announce(PeerInfoError::InfoHashInvalid)
                        .as_bytes()
                        .to_vec(),
                },
                Err(_) => get_error_response_for_announce(PeerInfoError::PoissonedLock)
                    .as_bytes()
                    .to_vec(),
            }
        }
        Err(error) => get_error_response_for_announce(error).as_bytes().to_vec(),
    }
}

fn extract_last_contents_of_response(
    buffer: &[u8],
    dic_torrents: &ArcMutexOfTorrents,
    json: &Arc<RwLock<Json>>,
    ip_port: &SocketAddr,
) -> Result<(Vec<u8>, String), CommunicationError> {
    let mut status_line = String::from(OK_URL);
    let contents = if buffer.starts_with(GET_URL) {
        fs::read(INDEX_HTML).map_err(|err| {
            CommunicationError::ReadingFilesToFillResponseContentError(err.to_string())
        })?
    } else if buffer.starts_with(STATS_URL) {
        fs::read(STATS_HTML).map_err(|err| {
            CommunicationError::ReadingFilesToFillResponseContentError(err.to_string())
        })?
    } else if buffer.starts_with(STYLE_URL) {
        fs::read(STYLE_CSS).map_err(|err| {
            CommunicationError::ReadingFilesToFillResponseContentError(err.to_string())
        })?
    } else if buffer.starts_with(DOCS_URL) {
        fs::read(DOCS_HTML).map_err(|err| {
            CommunicationError::ReadingFilesToFillResponseContentError(err.to_string())
        })?
    } else if buffer.starts_with(CODE_URL) {
        fs::read(CODE_JS).map_err(|err| {
            CommunicationError::ReadingFilesToFillResponseContentError(err.to_string())
        })?
    } else if buffer.starts_with(JSON_URL) {
        match json.read() {
            Ok(unlocked_json) => unlocked_json.get_json_string().as_bytes().to_vec(),
            Err(_err) => ERROR_500.as_bytes().to_vec(),
        }
    } else if buffer.starts_with(ANNOUNCE_URL) {
        //[TODO] Almacenar datos importantes [en .json?]
        get_response_details(buffer, dic_torrents, json, *ip_port)
    } else {
        status_line = String::from(ERR_URL);
        fs::read(ERROR_HTML).map_err(|err| {
            CommunicationError::ReadingFilesToFillResponseContentError(err.to_string())
        })?
    };
    Ok((contents, status_line))
}

fn handle_single_connection(
    mut stream: TcpStream,
    dic_torrents: ArcMutexOfTorrents,
    json: Arc<RwLock<Json>>,
    ip_port: SocketAddr,
) -> Result<(), CommunicationError> {
    let mut buffer = [0; 1024];
    stream
        .read(&mut buffer)
        .map_err(|err| CommunicationError::ReadingPeerSocket(err.to_string()))?;

    let (mut contents, status_line) =
        extract_last_contents_of_response(&buffer, &dic_torrents, &json, &ip_port)?;

    let result = if contents == ERROR_500.as_bytes().to_vec() {
        Err(CommunicationError::UnlockingMutexOfTorrents)
    } else {
        Ok(())
    };

    let mut response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n",
        status_line,
        contents.len(),
    )
    .as_bytes()
    .to_vec();

    response.append(&mut contents);

    stream
        .write_all(&response)
        .map_err(|err| CommunicationError::WritingResponse(err.to_string()))?;
    stream
        .flush()
        .map_err(|err| CommunicationError::WritingResponse(err.to_string()))?;

    result
}

pub fn general_communication(
    listener: TcpListener,
    mutex_of_torrents: ArcMutexOfTorrents,
    mutex_of_json: &Arc<RwLock<Json>>,
    global_shutdown: Arc<RwLock<bool>>,
    number_threads: usize,
) -> Result<(), CommunicationError> {
    let pool = ThreadPool::new(number_threads);

    loop {
        match listener.accept() {
            //Uso accept para obtener tambien la ip y el puerto de quien se conecto con el tracker
            Ok((stream, sock_addr)) => {
                let dic_copy: ArcMutexOfTorrents = Arc::clone(&mutex_of_torrents);
                let json_copy: Arc<RwLock<Json>> = Arc::clone(mutex_of_json);
                info!(
                    "Connected to  [ {} : {} ]",
                    sock_addr.ip(),
                    sock_addr.port()
                );
                let global_shutdown_copy = Arc::clone(&global_shutdown);
                pool.execute(move || {
                    match handle_single_connection(stream, dic_copy, json_copy, sock_addr) {
                        Ok(_) => (),
                        Err(CommunicationError::UnlockingMutexOfTorrents) => {
                            let _ = set_global_shutdown(&global_shutdown_copy);
                            error!("{}", CommunicationError::UnlockingMutexOfTorrents);
                        }
                        Err(err) => error!("{}", err),
                    }
                })
                .map_err(|err| CommunicationError::PoolExecutionError(err.to_string()))?;
            }
            Err(error) => {
                if error.kind() == ErrorKind::WouldBlock {
                    if is_global_shutdown_set(&global_shutdown) {
                        break;
                    };
                    //Por cada vez que no conecto espero 1 seg a la siguiente request
                    //Para no estar loopeando tan rapidamente y que explote la maquina.
                    thread::sleep(Duration::from_secs(1));
                } else {
                    set_global_shutdown(&global_shutdown)?;
                }
            }
        };
    }

    Ok(())
}
