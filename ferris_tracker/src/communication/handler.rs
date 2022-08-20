use std::{
    fs,
    io::{ErrorKind, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

use log::{error, info};

use crate::{
    data::{
        constants::*,
        peer_info::{get_error_response_for_announce, PeerInfo, PeerInfoError},
    },
    is_global_shutdown_set,
    thread_pool::ThreadPool,
    ArcMutexOfTorrents, ResultDyn, TrackerError,
};

fn get_response_details(
    buffer: &[u8],
    dic_torrents: &ArcMutexOfTorrents,
    ip_port: SocketAddr,
) -> ResultDyn<Vec<u8>> {
    let info_of_announced_peer = PeerInfo::new((*buffer).to_vec(), ip_port);

    let details = match info_of_announced_peer {
        Ok(info_of_announced_peer) => {
            let info_hash = info_of_announced_peer.get_info_hash();
            match dic_torrents.write() {
                Ok(mut unlocked_dic) => match unlocked_dic.get_mut(&info_hash) {
                    Some(torrent) => {
                        let response = torrent.get_bencoded_response_for_announce(
                            info_of_announced_peer.get_peer_id(),
                            info_of_announced_peer.is_compact(),
                        );
                        torrent
                            .add_peer(info_of_announced_peer.get_peer_id(), info_of_announced_peer);
                        response
                    }
                    None => get_error_response_for_announce(PeerInfoError::InfoHashInvalid)
                        .as_bytes()
                        .to_vec(),
                },
                Err(_) => return Err(Box::new(TrackerError::UnlockingMutexOfTorrents)), // Como este es error de nuestro server podriamos considerar cambiarlo a un error de codigo 500 por ej, sino el peer no se entera de nada y le cortamos de repente
            }
        }
        Err(error) => get_error_response_for_announce(error).as_bytes().to_vec(),
    };
    Ok(details)
}

fn handle_single_connection(
    mut stream: TcpStream,
    dic_torrents: ArcMutexOfTorrents,
    ip_port: SocketAddr,
) -> ResultDyn<()> {
    let mut buffer = [0; 1024];
    let _ = stream.read(&mut buffer);
    let mut status_line = OK_URL;

    let mut contents = if buffer.starts_with(GET_URL) {
        fs::read(INDEX_HTML)?
    } else if buffer.starts_with(STATS_URL) {
        fs::read(STATS_HTML)?
    } else if buffer.starts_with(STYLE_URL) {
        fs::read(STYLE_CSS)?
    } else if buffer.starts_with(DOCS_URL) {
        fs::read(DOCS_HTML)?
    } else if buffer.starts_with(CODE_URL) {
        fs::read(CODE_JS)?
    } else if buffer.starts_with(JSON_URL) {
        fs::read(JSON)?
    } else if buffer.starts_with(ANNOUNCE_URL) {
        //[TODO] Almacenar datos importantes [en .json?]
        get_response_details(&buffer, &dic_torrents, ip_port)?
    } else {
        status_line = ERR_URL;
        fs::read(ERROR_HTML)?
    };
    let mut response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n",
        status_line,
        contents.len(),
    )
    .as_bytes()
    .to_vec();

    response.append(&mut contents);

    stream.write_all(&response)?;
    stream.flush()?;
    Ok(())
}

pub fn general_communication(
    listener: TcpListener,
    mutex_of_torrents: ArcMutexOfTorrents,
    global_shutdown: Arc<RwLock<bool>>,
) {
    let pool = ThreadPool::new(4);

    loop {
        match listener.accept() {
            //Uso accept para obtener tambien la ip y el puerto de quien se conecto con el tracker
            Ok((stream, sock_addr)) => {
                let dic_copy: ArcMutexOfTorrents = Arc::clone(&mutex_of_torrents);
                info!(
                    "Connected to  [ {} : {} ]",
                    sock_addr.ip(),
                    sock_addr.port()
                );
                pool.execute(move || {
                    match handle_single_connection(stream, dic_copy, sock_addr) {
                        Ok(_) => (),
                        Err(error) => error!("{}", error), //Ver que hacer es casos de error
                    }
                });
            }
            Err(error) => {
                if error.kind() == ErrorKind::WouldBlock {
                    if is_global_shutdown_set(&global_shutdown) {
                        break;
                    };
                    //Por cada vez que no conecto espero 1 seg a la siguiente request
                    //Para no estar loopeando tan rapidamente y que explote la maquina.
                    thread::sleep(Duration::from_secs(1));
                }
            }
        };
    }
}
