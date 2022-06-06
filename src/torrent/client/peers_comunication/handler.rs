//! # Modulo de manejo de comunicación con peers
//! Este modulo contiene las funciones encargadas de controlar la logica de conexion e interaccion con todos los peers necesarios.
//!

use super::super::client_struct::Client;
use super::msg_logic_control::{interact_with_single_peer, MsgLogicControlError};
use std::fs;

/// Representa un tipo de estado de interaccion para saber si se debe
/// continuar o finalizar la misma
pub enum HandlerInteractionStatus {
    LookForAnotherPeer,
    FinishInteraction,
}

fn flush_data(client: &mut Client) -> Result<(), MsgLogicControlError> {
    fs::remove_dir_all("temp/torrent")
        .map_err(|err| MsgLogicControlError::RestartingDownload(format!("{:?}", err)))?;
    fs::create_dir("temp/torrent")
        .map_err(|err| MsgLogicControlError::RestartingDownload(format!("{:?}", err)))?;

    client
        .data_of_download
        .flush_data(client.torrent_file.total_size as u64);
    Ok(())
}

// FUNCION PRINCIPAL
/// Funcion encargada de manejar toda conexion y comunicación con todos los
/// peers que se hayan obtenido a partir de una respuesta de tracker e info
/// adicional del archivo .torrent correspondiente.
///
/// POR AHORA finaliza la comunicación cuando puede completar una pieza completa,
/// o en caso de error interno.
///
pub fn handle_general_interaction(client: &mut Client) -> Result<(), MsgLogicControlError> {
    // POR AHORA; LOGICA PARA COMPLETAR UNA PIEZA:
    let mut current_server_peer_index = 0;

    loop {
        if let Some(tracker_response) = &client.tracker_response {
            let max_server_peer_index = tracker_response.peers.len();
            if current_server_peer_index >= max_server_peer_index {
                return Err(MsgLogicControlError::ConectingWithPeer(
                    "No se pudo conectar con ningun peer para completar la pieza".to_string(),
                ));
            }
        };

        match interact_with_single_peer(client, current_server_peer_index) {
            Ok(HandlerInteractionStatus::LookForAnotherPeer) => {
                current_server_peer_index += 1;
                flush_data(client)?;
                continue;
            }
            Ok(HandlerInteractionStatus::FinishInteraction) => {
                return Ok(());
            }
            Err(MsgLogicControlError::ConectingWithPeer(_)) => {
                current_server_peer_index += 1;
                flush_data(client)?;
                continue;
            }
            Err(another_err) => {
                fs::remove_dir_all("temp/torrent").map_err(|err| {
                    MsgLogicControlError::RestartingDownload(format!("{:?}", err))
                })?;
                return Err(another_err);
            }
        };
    }
}

// =================================================================================================================

//
// LOGICA PARA GENERALIZAR CUANDO HAYA MAS DE UN PEER:

// use std::sync::mpsc;
// use std::thread;
// use std::sync::Arc;
// use std::sync::Mutex;

// pub struct ThreadPool {
//     workers: Vec<Worker>,
//     sender: mpsc::Sender<Job>,
// }

// type Job = Box<dyn FnOnce() + Send + 'static>;

// impl ThreadPool {

//     pub fn new(size: usize) -> ThreadPool {
//         assert!(size > 0);

//         let (sender, receiver) = mpsc::channel();

//         let receiver = Arc::new(Mutex::new(receiver));

//         let mut workers = Vec::with_capacity(size);

//         for id in 0..size {
//             workers.push(Worker::new(id, Arc::clone(&receiver)));
//         }

//         ThreadPool { workers, sender }
//     }

//     // --snip--

//     pub fn execute<F>(&self, f: F)
//     where
//         F: FnOnce() + Send + 'static,
//     {
//         let job = Box::new(f);

//         self.sender.send(job).unwrap();
//     }
// }

// struct Worker {
//     id: usize,
//     thread: thread::JoinHandle<()>,
// }

// impl Worker {
//     fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
//         let thread = thread::spawn(move || loop {
//             let job = receiver.lock().unwrap().recv().unwrap();

//             println!("Worker {} got a job; executing.", id);

//             job();
//         });

//         Worker { id, thread }
//     }
// }
