# Proyecto Taller de Programación I (1C-2022) - FA-torrent

<p align="center">

---


<br>
<br>
  <img src="https://www.estudiaradistancia.com.ar/logos/original/logo-universidad-de-buenos-aires.webp" height=150 />
  <img  src="https://confedi.org.ar/wp-content/uploads/2020/09/fiuba_logo.jpg" height="150">
<br>
<br>
  <img src="https://aws1.discourse-cdn.com/business5/uploads/rust_lang/original/2X/9/9f76ef5e791e27deaaafbca2a3bea35d63e165c8.gif" />

---

</p>

## Grupo - Ferris Appreciators

### Integrantes

- Axel Aparicio
- Luciano Gamberale
- Erick Martinez
- Miguel Vasquez

---

## Objetivo del Proyecto (PRE-AGREGADO)

El objetivo del proyecto es implementar un Cliente de BitTorrent con funcionalidades acotadas, detalladas [aquí](https://taller-1-fiuba-rust.github.io/proyecto/22C1/proyecto.html).

## Objetivo del agregado (final)

Se tiene como objetivo de funcionalidad agregada implementar un Tracker HTTP de BitTorrent con funcionalidades acotadas, y su visualización en un sitio web por medio del browser. Funcionalidades detalladas [aquí](https://taller-1-fiuba-rust.github.io/proyecto/22C1/proyecto_final_1C2022.html).

---

## Ejecución

***(WIP)***

En general, los distintos `<tipo_de_log>` son: `info, debug, trace`.

### Cliente *Ferris Torrent*

```bash
RUST_LOG=<tipo_de_log> cargo run -p ferris_torrent -- <ARGS: archivos_torrent / path_a_directorio_con_torrents>
```

### *Ferris Tracker*

```bash
RUST_LOG=<tipo_de_log> cargo run -p ferris_tracker
```
### Tests

#### Generales (Incluye todos los packages del Workspace de Cargo.toml)

```bash
cargo test
```

#### Solo tests de *Ferris Torrent*

```bash
cargo test -p ferris_torrent
```

Adicionalmente si se quiere correr un solo test específico o un modulo con tests, ejecutar:
```bash
cargo test -p ferris_torrent -- --test <nombre_del_modulo/test>
```

#### Solo tests de *Ferris Tracker*

```bash
cargo test -p ferris_tracker
```

---


## Funcionalidad soportada

### Primera versión (Release: *checkpoint*)

- Recibir por linea de comandos la ruta de un archivo .torrent
- Dicho .torrent es leído y decodificado según el estándar y su información almacenada.
- Se conecta al Tracker obtenido en el .torrent y se comunica con el mismo, decodifica su respuesta y obtiene una lista de peers.
- Se conecta con un peer y realiza la comunicación completa con el mismo para poder descargar una pieza del torrent.
- La pieza descargada es validada internamente, pero puede verificarse también por medio del script sha1sum de linux.

### Segunda versión (Release: *Entrega final de cursada*)

- Permite recibir por linea de comandos la ruta de uno o más archivos ".torrent"; o un la ruta a un directorio con ellos.
- Se ensamblan las piezas de cada torrent para obtener el archivo completo.
- Funciona como server, es decir, responde a requests de piezas.
- Cuenta con interfaz gráfica.
- Cuénta con un logger en archivos que indica cuándo se descargan las piezas (y adicionalmente se loggean errores importantes).
- Se pueden customizar el puerto en el que se escuchan peticiones, directorio de descargas y de logs mediante un archivo config.txt
- Puede descargar más de un torrent concurrentemente, y por cada uno de esos torrents puede descargar más de una pieza de la misma

