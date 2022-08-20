//! # FA-torrent
//! ## Grupo - Ferris Appreciators
//! ### Objetivo del agregado
//!
//! El objetivo del agregado es implementar un Cliente de BitTorrent con funcionalidades acotadas, detalladas [aquí](https://taller-1-fiuba-rust.github.io/proyecto/22C1/proyecto.html).
//!
//!
//! Primera versión (checkpoint release):
//!
//! - Recibir por linea de comandos la ruta de un archivo .torrent
//! - Dicho .torrent es leído y decodificado según el estándar y su información almacenada.
//! - Se conecta al Tracker obtenido en el .torrent y se comunica con el mismo, decodifica su respuesta y obtiene una lista de peers.
//! - Se conecta con un peer y realiza la comunicación completa con el mismo para poder descargar una pieza del torrent.
//! - La pieza descargada es validada internamente, pero puede verificarse también por medio del script sha1sum de linux.
//!
//! Segunda versión:
//!
//! - Permite recibir por linea de comandos la ruta de uno o más archivos ".torrent"; o un la ruta a un directorio con ellos.
//! - Se ensamblan las piezas de cada torrent para obtener el archivo completo.
//! - Funciona como server, es decir, responde a requests de piezas.
//! - Cuenta con interfaz gráfica.
//! - Cuénta con un logger en archivos que indica cuándo se descargan las piezas (y adicionalmente se loggean errores importantes).
//! - Se pueden customizar el puerto en el que se escuchan peticiones, directorio de descargas y de logs mediante un archivo config.txt
//! - Puede descargar más de un torrent concurrentemente, y por cada uno de esos torrents puede descargar más de una pieza de la misma forma. A su vez puede ser server de otros peers.
//!
//!

pub mod communication;
pub mod data;
pub mod thread_pool;

use std::{
    collections::HashMap,
    error::Error,
    fmt,
    net::TcpListener,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
};

use log::info;

use data::torrent_info::TorrentInfo;

type ArcMutexOfTorrents = Arc<RwLock<HashMap<Vec<u8>, TorrentInfo>>>;
type ResultDyn<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub enum TrackerError {
    UnlockingMutexOfTorrents,
}

impl fmt::Display for TrackerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for TrackerError {}

fn init_torrents() -> ArcMutexOfTorrents {
    //Creo un torrent generico para hacer pruebas
    let mut dic_torrents = HashMap::new();
    dic_torrents.insert(
        "abcdefghijklmn123456".as_bytes().to_vec(),
        TorrentInfo::new("abcdefghijklmn123456".as_bytes().to_vec()),
    );
    // ...
    // ...

    //RwLock de un diccionario que contiene los TorrentInfo
    Arc::new(RwLock::new(dic_torrents))
}

fn init_handler_for_quit_input(global_shutdown: Arc<RwLock<bool>>) -> JoinHandle<()> {
    let exit_command = String::from("q\n");
    info!("Waiting for input");
    thread::spawn(move || loop {
        let mut command = String::new();
        let _ = std::io::stdin().read_line(&mut command);
        if command == exit_command {
            info!("Executing quit command");
            let _ = set_global_shutdown(&global_shutdown); // Revisar que hacer con el error que surge de aca.
            break;
        }
    })
}

fn is_global_shutdown_set(global_shutdown: &Arc<RwLock<bool>>) -> bool {
    if let Ok(mutex_sutdown) = global_shutdown.read() {
        *mutex_sutdown
    } else {
        true // Si el global shutdown está poisoned, hay que cortar todo igual
    }
}

fn set_global_shutdown(global_shutdown: &Arc<RwLock<bool>>) -> ResultDyn<()> {
    let mut global_shutdown = global_shutdown.write().map_err(|err| format!("{}", err))?;
    *global_shutdown = true;
    Ok(())
}

///
/// FUNCION PRINCIPAL PARA LA EJECUCION DEL PROGRAMA
///
///
///
/// ... (Despues se puede ver si permitimos tener una especie de tracker dinamico con torrents adicionales)
/// Devuelve un Error si hubo algún problema durante todo el proceso.
///
pub fn run() -> ResultDyn<()> {
    // Hay que ver a lo ultimo si se pueden hacer refactors sobre los errores asi no devolvemos Box dyn

    pretty_env_logger::init();
    info!("tracker init");

    let global_shutdown = Arc::new(RwLock::new(false));

    let mutex_of_torrents: ArcMutexOfTorrents = init_torrents();

    let join_hander = init_handler_for_quit_input(Arc::clone(&global_shutdown));

    // Nota (Miguel): Por las dudas al pasarlo al otro lado, despues usar el try bind del tp viejo.
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    let _ = listener.set_nonblocking(true);
    info!("Listening...");
    communication::handler::general_communication(listener, mutex_of_torrents, global_shutdown);

    let _ = join_hander.join();

    Ok(())
}
