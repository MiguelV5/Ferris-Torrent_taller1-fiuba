# Proyecto Taller de Programación I (1C-2022) - FA-torrent

<p align="center">
  <img src="https://aws1.discourse-cdn.com/business5/uploads/rust_lang/original/2X/9/9f76ef5e791e27deaaafbca2a3bea35d63e165c8.gif" />
</p>

## Grupo - Ferris Appreciators

### Integrantes

- Axel Aparicio
- Luciano Gamberale
- Erick Martinez
- Miguel Vasquez

<!-- ### Presentación - Entrega parcial

Haga click [aquí](link) para ingresar a la presentación -->

### Objetivo del Proyecto

El objetivo del proyecto es implementar un Cliente de BitTorrent con funcionalidades acotadas, detalladas [aquí](https://taller-1-fiuba-rust.github.io/proyecto/22C1/proyecto.html).

En este momento el proyecto sigue en desarrollo.

### Funcionalidad soportada actualmente

En su versión actual, el cliente soporta:

- Recibir por linea de comandos la ruta de un archivo torrent
- Dicho .torrent es leído y decodificado según el estándar y su información almacenada.
- Se conecta al Tracker obtenido en el .torrent y se comunica con el mismo, decodifica su respuesta y obtiene una lista de peers.
- Se conecta con un peer y realiza la comunicación completa con el mismo para poder descargar una pieza del torrent.
- La pieza descargada es validada internamente, pero puede verificarse también por medio del script sha1sum de linux.

<!-- ### Ejecución

Por medio de:

```bash
cargo run <ARCHIVO_TORRENT>
``` -->

[comment]: # (POSTERIORMENTE CAMBIAR ESTO^ A: <ARCHIVO_TORRENT/DIRECTORIO_CON_ARCHIVOS_TORRENT>)

[comment]: # (La linea siguiente es para descomentar despues cuando se tenga la funcionalidad)

<!-- El cliente soporta ser configurado en cuanto al directorio destino de la descarga y al archivo en el que se desee loggear por medio de la modificación del archivo configuration.txt, de la forma: destination_path:<path_deseado>  logging_path:<path_deseado>  -->

<!-- ### Diagramas

- [Representacion de estructuras](https://lucid.app/lucidchart/27229976-e32f-4112-acb3-d6b51859f301/edit?viewport_loc=-367%2C669%2C3017%2C1200%2C0_0&invitationId=inv_b76263b7-671f-4c41-8125-54379d933991#)

- [Representación de interaccion en la arquitectura](https://lucid.app/lucidchart/2aff6563-7a7d-4c39-9f24-f0fc439dcde2/edit?viewport_loc=21%2C492%2C1767%2C703%2C0_0&invitationId=inv_5664b582-66bb-4826-a6d2-1e0c4a7abfbb#) -->
