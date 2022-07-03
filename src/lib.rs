//! # FA-torrent
//! ## Grupo - Ferris Appreciators
//! ### Objetivo del Proyecto
//!
//! El objetivo del proyecto es implementar un Cliente de BitTorrent con funcionalidades acotadas, detalladas [aquí](https://taller-1-fiuba-rust.github.io/proyecto/22C1/proyecto.html).
//!
//! En este momento el proyecto sigue en desarrollo.
//!
//! En su versión actual, el cliente soporta:
//!
//! - Recibir por linea de comandos la ruta de un archivo .torrent
//! - Dicho .torrent es leído y decodificado según el estándar y su información almacenada.
//! - Se conecta al Tracker obtenido en el .torrent y se comunica con el mismo, decodifica su respuesta y obtiene una lista de peers.
//! - Se conecta con un peer y realiza la comunicación completa con el mismo para poder descargar una pieza del torrent.
//! - La pieza descargada es validada internamente, pero puede verificarse también por medio del script sha1sum de linux.
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

/// Funcion principal de ejecución del programa.
/// (En su version actual) Realiza todo lo necesario para descargar una pieza válida a partir de un archivo .torrent dado por consola.
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
