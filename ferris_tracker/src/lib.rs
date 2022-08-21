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

pub mod tracker;

use std::{
    collections::HashMap,
    error::Error,
    fmt,
    fs::{self, ReadDir},
    net::TcpListener,
    path::Path,
    sync::{Arc, RwLock},
    thread::{self, JoinHandle},
};

use log::{debug, error, info};
use shared::medatada_analyzer;

use crate::tracker::{
    communication::{
        self,
        handler::{set_global_shutdown, CommunicationError},
    },
    config_file_tracker,
    data::torrent_info::TorrentInfo,
};

type ArcMutexOfTorrents = Arc<RwLock<HashMap<Vec<u8>, TorrentInfo>>>;
type ResultDyn<T> = Result<T, Box<dyn Error>>;

#[derive(PartialEq, Eq, Debug)]
pub enum TrackerError {
    UnlockingMutexOfTorrents,
    JoiningQuitInput,
    CommsError(CommunicationError),
    NotFoundTorrents,
    NotFoundTorrentsDirectory,
    Folder(String),
    CreatingTorrent(String),
}

impl fmt::Display for TrackerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for TrackerError {}

///
/// Funcion encargada de analizar la ruta enviada por consola y generar una lista con todos los archivos .torrent que
/// pertenecen a ese directorio o archivo.
/// En caso de ser un directorio, la lista posee cada una de las rutas a cada archivo . torrent.
/// En caso de ser un archivo, tambien se lo coloca en una lista y se lo retorna como único elemento de la lista.
///
fn create_list_files(torrents_path: String) -> Result<Vec<String>, TrackerError> {
    let mut list_files = vec![];

    let path_args = Path::new(&torrents_path);
    if path_args.is_dir() {
        match fs::read_dir(path_args) {
            Ok(folder) => add_files_from_folder(&mut list_files, folder)?,
            Err(_error) => return Err(TrackerError::NotFoundTorrentsDirectory),
        }
    } else {
        error!("No se encontro la carpeta ingresada");
        return Err(TrackerError::NotFoundTorrentsDirectory);
    }

    if list_files.is_empty() {
        error!("No ingreso archivo/s en el directorio");
        return Err(TrackerError::NotFoundTorrents);
    }
    Ok(list_files)
}

fn add_files_from_folder(list: &mut Vec<String>, folder: ReadDir) -> Result<(), TrackerError> {
    for file in folder {
        match file {
            Ok(file_ok) => {
                let str_file = file_ok.path().display().to_string();
                list.push(str_file);
            }
            Err(error) => return Err(TrackerError::Folder(error.to_string())),
        }
    }
    Ok(())
}

fn init_torrents(torrents_path: String) -> Result<ArcMutexOfTorrents, TrackerError> {
    let list_torrents = create_list_files(torrents_path)?;
    let mut dic_torrents = HashMap::new();
    for torrent_file in list_torrents {
        let file_path = torrent_file.clone();
        debug!("Archivo ingresado: {}", file_path);
        info!("Archivo ingresado con exito");

        let torrent_file = medatada_analyzer::create_torrent(&file_path)
            .map_err(|_| TrackerError::CreatingTorrent(file_path))?;
        let torrent_info_hash = torrent_file.get_info_hash();
        dic_torrents.insert(
            torrent_info_hash.clone(),
            TorrentInfo::new(torrent_info_hash.clone()),
        );
    }

    //RwLock de un diccionario que contiene los TorrentInfo
    Ok(Arc::new(RwLock::new(dic_torrents)))
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

///
/// FUNCION PRINCIPAL PARA LA EJECUCION DEL PROGRAMA
///
///
///
/// ... (Despues se puede ver si permitimos tener una especie de tracker dinamico con torrents adicionales)
/// Devuelve un Error si hubo algún problema durante todo el proceso.
///
pub fn run() -> ResultDyn<()> {
    pretty_env_logger::init();
    info!("tracker init");

    let global_shutdown = Arc::new(RwLock::new(false));

    let config_data = config_file_tracker::ConfigFileData::new("config.txt")?;
    info!("Archivo de configuración leido y parseado correctamente");

    let mutex_of_torrents: ArcMutexOfTorrents = init_torrents(config_data.get_torrents_path())?;

    let join_hander = init_handler_for_quit_input(Arc::clone(&global_shutdown));

    // Nota (Miguel): Por las dudas al pasarlo al otro lado, despues usar el try bind del tp viejo.
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    let _ = listener.set_nonblocking(true);
    info!("Listening...");
    communication::handler::general_communication(
        listener,
        mutex_of_torrents,
        global_shutdown,
        config_data.get_number_of_threads(),
    )
    .map_err(TrackerError::CommsError)?;

    join_hander
        .join()
        .map_err(|_err| TrackerError::JoiningQuitInput)?;

    Ok(())
}
