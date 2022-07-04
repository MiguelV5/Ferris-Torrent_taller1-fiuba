//! # FA-torrent
//! ## Grupo - Ferris Appreciators
//! ### Objetivo del Proyecto
//!
//! El objetivo del proyecto es implementar un Cliente de BitTorrent con funcionalidades acotadas, detalladas [aquí](https://taller-1-fiuba-rust.github.io/proyecto/22C1/proyecto.html).
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

pub mod torrent;

use crate::torrent::{
    torrent_handler::{self},
    user_interface::builder_app,
};
use gtk::prelude::ApplicationExtManual;
use log::info;
use std::{
    error::Error,
    sync::{Arc, RwLock},
};

fn set_global_shut_down(global_shut_down: Arc<RwLock<bool>>) -> Result<(), Box<dyn Error>> {
    let mut global_shut_down = global_shut_down.write().map_err(|err| format!("{}", err))?;
    *global_shut_down = true;
    Ok(())
}

///
///  FUNCION PRINCIPAL PARA LA EJECUCION DEL PROGRAMA
/// A partir de una ruta enviada por consola (la ruta puede corresponder a un archivo .torrent en particular o a un directorio con archivos .torrent) se descarga/descargan todos los archivos correspondientes a cada uno de los .torrent dentro de una carpeta especificada dentro del archivo de configuración. Además, es necesario que en el archivo de configuracion se suministre la ruta del directorio de los logs a crear y el puerto en donde se podran escuchar conexiones externas.  
/// Devuelve un Error si hubo algún problema durante todo el proceso.
///
pub fn run() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    info!("Iniciando el programa");

    let (application, ui_sender) = builder_app::build_app();
    let global_shut_down = Arc::new(RwLock::new(false));

    let (torrent_handler_1, torrent_handler_2) =
        torrent_handler::handle_all_torrents(ui_sender, &global_shut_down)?;

    let empty_vec: Vec<&str> = vec![];
    application.run_with_args(&empty_vec);

    set_global_shut_down(global_shut_down)?;

    let torrent_result_1 = torrent_handler_1
        .join()
        .map_err(|_| ("[TorrentHandlerError] Join handle error".to_string()));
    let torrent_result_2 = torrent_handler_2
        .join()
        .map_err(|_| ("[TorrentHandlerError] Join handle error".to_string()));

    torrent_result_1??;
    torrent_result_2??;

    Ok(())
}
