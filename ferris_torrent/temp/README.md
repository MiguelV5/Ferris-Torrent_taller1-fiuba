# Nota importante de este directorio

***Su uso es exclusivo de los tests. Para ejecutiones normales se usa el directorio temp en el directorio root del proyecto (fa-torrent)***
Esto se debe a que:
- Al ejecutar los tests, el "directorio relativo" actual (para crear/destruir y almacenar) es ferris_torrent. 
- Al ejecutar EL PACKAGE (`cargo run -p ferris_torrent ...`), el "directorio relativo" actual (para crear/destruir y almacenar) es el directorio root del proyecto (fa-torrent).

Esta diferencia causa problemas a la hora de verificar el comportamiento de algunos tests, por lo cuál se decidió separar un directorio temp para uso en tests y otro para uso del programa normalmente desde el dir root.
