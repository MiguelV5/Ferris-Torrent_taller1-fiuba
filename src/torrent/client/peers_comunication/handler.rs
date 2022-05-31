#![allow(dead_code)]
use super::super::client_struct::Client;
use super::msg_logic_control::{interact_with_single_peer, MsgLogicControlError};

// (ESTAS SON LAS NOTAS VIEJAS QUE ESTABAN EN EL OTRO ARCHIVO PERO LAS TRAJE PARA ACA):
// Notas
// Miguel y Luciano: En el google docs se tiene que antes de establecer conexion con peers se debe ver como viene la
//  lista de peers segun clave compact de la respuesta del tracker.
//  Esto en realidad es, justamente, responsabilidad de lo que se encargue de recibir la info del tracker.
//
// Miguel: Estuve releyendo el proceso y esta funcion en realidad seria algo como la entrada principal a toda la logica de conexion.
//  O sea, necesitamos una funcion (esta misma) que deberia establecer y manejar la conexion con todos los peers (o con los necesarios)
//  y luego hacer algo asi como llamar a una funcion que se encargue de hacer todo el protocolo de leecher con distintos peers en threads.
//

pub fn handle_general_interaction(client: &mut Client) -> Result<(), MsgLogicControlError> {
    // LOGICA PARA GENERALIZAR CUANDO HAYA MAS DE UN PEER:

    //...
    // POR AHORA; MIENTRAS QUE SE REQUIERE SOLO UN PEER A COMPLETAR UNA PIEZA:
    interact_with_single_peer(client, 0)
    // (falta, pero ir viendo nota de msg_logic_control.rs:react_according_to_the_received_msg: caso Piece)
}
