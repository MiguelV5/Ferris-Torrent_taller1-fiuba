use std::env;

use fa_torrent::torrent::client::client_struct::Client;
use log::{debug, error, info, trace};
use std::error::Error;

mod torrent;

fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    info!("Iniciando el programa");
    //[POR HACER: Crear struct/funcion que se encargue de el manejo de archivos ingresados]
    let file_str = match env::args().nth(1) {
        Some(file) => file,
        None => {
            error!("No se ingreso archivo por terminal");
            return Ok(());
        }
    };
    debug!("Archivo ingresado: {}", file_str);
    trace!("Archivo ingresado con exito");
    info!("Creacion de la estructura Client");
    let mut client_struct = Client::new(&file_str)?;
    info!("El Client fue creado exitosamente");
    info!("Inicio de comunicacion con tracker mediante Client");
    client_struct.init_communication()?;
    info!("Comunicacion con el tracker exitosa");
    //Queda por terminar
    Ok(())
}
