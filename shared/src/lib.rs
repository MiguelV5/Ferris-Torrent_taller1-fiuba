// Mi idea es traer a este package las cosas que comparten el tracker y el torrent client para evitar dependencias repetidas (Por ej si se planteaba como en el primer commit de esta rama el problema es que para ejecutar el tracker se compilaba TODO lo del cliente incluido gtk y es horrible)

pub mod parsers;
// pub mod http_hander; //(No estoy seguro si esto es compartido pero me parece que si, revisar)
