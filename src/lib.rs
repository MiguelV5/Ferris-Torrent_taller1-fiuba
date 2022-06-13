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
    client::peers_comunication,
    data::torrent_status::TorrentStatus,
    local_peer::{communicate_with_tracker, create_torrent},
};

use log::{debug, error, info, trace};
use std::{env, error::Error};

/// Funcion principal de ejecución del programa.
/// (En su version actual) Realiza todo lo necesario para descargar una pieza válida a partir de un archivo .torrent dado por consola.
/// Devuelve un Error si hubo algún problema durante todo el proceso.
///
pub fn run() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    info!("Iniciando el programa");

    let file_path = match env::args().nth(1) {
        Some(file) => file,
        None => {
            error!("No se ingreso archivo por terminal");
            return Ok(());
        }
    };
    debug!("Archivo ingresado: {}", file_path);
    info!("Archivo ingresado con exito");

    info!("Creacion de la estructura Client");
    //let mut client_struct = LocalPeer::new()?;

    //===========================

    let torrent_file = create_torrent(&file_path)?;
    trace!("TorrentFileData creado y almacenado dentro del Client");
    let torrent_size = torrent_file.get_total_length() as u64;
    let mut torrent_status = TorrentStatus::new(torrent_size, torrent_file.total_amount_of_pieces);

    //===========================

    info!("El Client fue creado exitosamente");

    info!("Inicio de comunicacion con tracker mediante Client");
    let tracker_response = communicate_with_tracker(torrent_file.clone())?;
    info!("Comunicacion con el tracker exitosa");

    info!("Inicio de comunicacion con peers.");
    peers_comunication::handler::handle_general_interaction_as_client(
        &torrent_file,
        &tracker_response,
        &mut torrent_status,
    )?;
    info!("Se descargó exitosamente una pieza.");

    Ok(())
}
