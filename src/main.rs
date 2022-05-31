use std::env;

use fa_torrent::torrent::client::client_struct::Client;

mod torrent;

fn main() {
    println!("Iniciando el programa...");
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Error de la aplicación");
    }
    let mut client_struct = match Client::new(args[1].as_str()) {
        Ok(client) => client,
        Err(error) => return println!("Error de la aplicación: {:?}", error),
    };
    match client_struct.init_communication() {
        Ok(_) => (),
        Err(error) => return println!("Error de la aplicación: {:?}", error),
    }
}
