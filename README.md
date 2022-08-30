# Proyecto Taller de Programación I (1C-2022) - Ferris-torrent (Agregado: Ferris-Tracker)


---


<p align="center">
<br>
<br>
  <img src="https://www.estudiaradistancia.com.ar/logos/original/logo-universidad-de-buenos-aires.webp" height=150 />
  <img  src="https://confedi.org.ar/wp-content/uploads/2020/09/fiuba_logo.jpg" height="150">
<br>
<br>
  <img src="https://aws1.discourse-cdn.com/business5/uploads/rust_lang/original/2X/9/9f76ef5e791e27deaaafbca2a3bea35d63e165c8.gif" />
</p>

---


## Grupo - Ferris Appreciators

### Integrantes

- Axel Aparicio
- Luciano Gamberale
- Erick Martinez
- Miguel Vasquez

---

## Objetivo del Proyecto (PRE-AGREGADO [Entrega final de Cursada])

El objetivo del proyecto es implementar un Cliente de BitTorrent con funcionalidades acotadas, detalladas [aquí](https://taller-1-fiuba-rust.github.io/proyecto/22C1/proyecto.html).

## Objetivo del agregado (AGREGADO [Fechas de final Jul-Ago])

Se tiene como objetivo de funcionalidad agregada implementar un Tracker HTTP de BitTorrent con funcionalidades acotadas, y su visualización en un sitio web por medio del browser. Funcionalidades detalladas [aquí](https://taller-1-fiuba-rust.github.io/proyecto/22C1/proyecto_final_1C2022.html).

---

## Ejecución


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

(**Nota:** Todos los releases referidos se encuentran en el repositorio original)

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

### Tercera versión (Release: *Entrega final (Agregado 1c 2022)*)

- Implementación de Tracker que recibe y responde correctamente desde localhost:7878, sea:
    - En browser con endpoints index, /stats y /docs
    - En comunicación directa desde un cliente de BitTorrent (usando el endpoint /announce)
- Responde el endpoint /stats mostrando estadísticas sobre peers conectados, peers con descarga completa y cantidad de torrents en el tracker. Estas estadísticas son mostradas en el sitio web HTML que puede ser accedido desde un browser. Esta página permite la visualización filtrada de las estadísticas
de acuerdo a períodos fijos de tiempo (última hora, últimas 5 horas, último día, últimos 3 días) y con determinadas frecuencias (horas, minutos).
- Responde el endpoint /announce correctamente a uno o más peers determinados, siguiendo la documentación encontrada en el endpoint /docs.
- Para acceso a los distintos endpoints simplemente ejecutar el tracker y abrir en el browser la página en localhost:7878. Desde allí se provee la interfáz front para ingresar a los distintos endpoints mencionados.

(No se agrega más funcionalidad al Cliente Ferris-Torrent)
