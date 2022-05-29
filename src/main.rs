use std::env;
use std::error::Error;

use fa_torrent::start_torrent_process;
mod torrent;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Iniciando el programa...");
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Error de la aplicación");
        return Err("Esperando un path".into());
    }
    if let Err(err) = start_torrent_process(args[1].as_str()) {
        eprintln!("Error de la aplicación: {}", err);
        return Err(err);
    };

    Ok(())
}
