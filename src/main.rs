use std::env;

use fa_torrent::torrent::client::{client_struct::Client, peers_comunication};

use log::{debug, error, info};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    info!("Iniciando el programa");
    //[POR HACER: Crear struct/funcion que se encargue de el manejo de archivos ingresados]
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
    let mut client_struct = Client::new(&file_path)?;
    info!("El Client fue creado exitosamente");

    info!("Inicio de comunicacion con tracker mediante Client");
    client_struct.init_communication()?;
    info!("Comunicacion con el tracker exitosa");

    info!("Inicio de comunicacion con peers.");
    peers_comunication::handler::handle_general_interaction(&mut client_struct)?;
    info!("Se descargó exitosamente una pieza.");

    Ok(())
}
