use std::{
    error::Error,
    fmt,
    fs::{File, OpenOptions},
    io::Write,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use log::error;

#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Logger {
    log_path: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LogError {
    CanNotCreateFile,
    CanNotWrite,
    CanNotCloseFile,
    CanNotOpenFile(String),
    CanNotJoin,
}

impl fmt::Display for LogError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for LogError {}

impl Logger {
    pub fn new(log_path_dir: String, torrent_name: String) -> Result<Logger, LogError> {
        let log_path = format!("{}/{}-logs.txt", log_path_dir, torrent_name);
        Ok(Logger { log_path })
    }

    /// Inicializa el logueo y devuelve una tupla con el sender para que sea utilizado para el
    /// envío de mensajes y un handler para realizar el join.
    /// El receiver queda esperando que se le envíe contenido en un thread aparte, e intentará
    /// escribirlo en el archivo de logs. Solo se debe inicializar el log una vez, pero pueden
    /// existir múltiples senders clonando el sender retornado.
    ///
    /// #Ejemplo
    /// ```
    /// use fa_torrent::torrent::logger::*;
    ///
    /// //Inicializa el sender y el receiver, y ademàs se obtiene el handle para hacer join
    /// let logger = Logger::new("temp/logs".to_string(), "mitorrent.torrent".to_string()).unwrap();
    /// let (sender1, handle) = logger.init_logger().unwrap();
    /// //Obtener un segundo sender
    /// let sender2 = sender1.clone();
    /// sender1.send("Descargué la pieza 1".to_string());
    /// sender2.send("Descargué la pieza 2".to_string());
    ///
    /// //Join final, para esperar que el hilo termine de loguear
    /// drop(sender1);
    /// drop(sender2);
    /// handle.join();
    ///
    /// //Remuevo el archivo test que se genera
    /// std::fs::remove_file(logger.get_log_path());
    /// ```
    pub fn init_logger(&self) -> Result<(Sender<String>, JoinHandle<()>), LogError> {
        let (sender, receiver): (Sender<String>, Receiver<String>) = mpsc::channel();
        let log_path = self.get_log_path();
        let mut logger_file = open_logger(&log_path)?;
        let handle = thread::spawn(move || {
            for content in receiver.iter() {
                if let Err(err) = writeln!(logger_file, "{}", &content) {
                    //Informo el error por consola, pero no corto la ejecución del programa sólo por no
                    //poder escribir en el archivo de logs
                    error!(
                        "No se pudo escribir el mensaje {} en el archivo de logs {}, error: {}",
                        &content, &log_path, &err
                    )
                }
            }
        });

        Ok((sender, handle))
    }

    ///Log path getter
    pub fn get_log_path(&self) -> String {
        self.log_path.clone()
    }
}

fn open_logger(log_path: &String) -> Result<File, LogError> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|err| LogError::CanNotOpenFile(format!("{}", err)))?;

    Ok(file)
}

mod tests {
    #![allow(unused_imports, dead_code)]
    use core::time;
    use std::{
        fs,
        io::{BufRead, BufReader, Read},
    };

    use super::*;

    fn read_log_file(logger: &Logger) -> Result<Vec<String>, LogError> {
        let file = File::open(logger.get_log_path())
            .map_err(|err| LogError::CanNotOpenFile(format!("{}", err)))?;
        let buf = BufReader::new(file);
        let result: Vec<String> = buf
            .lines()
            .map(|l| l.expect("Could not parse line"))
            .collect();
        Ok(result)
    }

    #[test]
    fn write_in_log_with_one_sender_ok() -> Result<(), LogError> {
        let logger = Logger::new(
            "temp/logs".to_string(),
            "write_in_log_with_one_sender_ok.torrent".to_string(),
        )?;
        let (sender1, handle) = logger.init_logger()?;
        sender1
            .send("test_msg".to_string())
            .map_err(|err| LogError::CanNotOpenFile(format!("{}", err)))?;

        drop(sender1);
        handle.join().map_err(|_| LogError::CanNotJoin)?;

        let result = read_log_file(&logger)?;
        fs::remove_file(logger.get_log_path()).map_err(|_| LogError::CanNotCloseFile)?;

        assert_eq!(result.len(), 1);
        assert_eq!(result.get(0), Some(&"test_msg".to_string()));

        Ok(())
    }

    #[test]
    fn write_in_log_with_one_sender_with_multiple_messages_ok() -> Result<(), LogError> {
        let logger = Logger::new(
            "temp/logs".to_string(),
            "write_in_log_with_one_sender_with_multiple_messages_ok.torrent".to_string(),
        )?;
        let (sender1, handle) = logger.init_logger()?;
        for i in 0..3 {
            sender1
                .send("test_msg".to_string() + i.to_string().as_str())
                .map_err(|err| LogError::CanNotOpenFile(format!("{}", err)))?;
        }

        drop(sender1);
        handle.join().map_err(|_| LogError::CanNotJoin)?;

        let result = read_log_file(&logger)?;
        fs::remove_file(logger.get_log_path()).map_err(|_| LogError::CanNotCloseFile)?;

        assert_eq!(result.len(), 3);
        assert_eq!(result.get(0), Some(&"test_msg0".to_string()));
        assert_eq!(result.get(1), Some(&"test_msg1".to_string()));
        assert_eq!(result.get(2), Some(&"test_msg2".to_string()));

        Ok(())
    }

    #[test]
    fn write_in_log_with_two_sender_ok() -> Result<(), LogError> {
        let logger = Logger::new(
            "temp/logs".to_string(),
            "write_in_log_with_two_sender_ok.torrent".to_string(),
        )?;
        let (sender1, handle) = logger.init_logger()?;
        sender1
            .send("test_msg".to_string())
            .map_err(|err| LogError::CanNotOpenFile(format!("{}", err)))?;

        let sender2 = sender1.clone();
        sender2
            .send("test_sender2".to_string())
            .map_err(|err| LogError::CanNotOpenFile(format!("{}", err)))?;

        drop(sender1);
        drop(sender2);
        handle.join().map_err(|_| LogError::CanNotJoin)?;

        let result = read_log_file(&logger)?;
        fs::remove_file(logger.get_log_path()).map_err(|_| LogError::CanNotCloseFile)?;

        assert_eq!(result.len(), 2);
        assert_eq!(result.get(0), Some(&"test_msg".to_string()));
        assert_eq!(result.get(1), Some(&"test_sender2".to_string()));

        Ok(())
    }
}
