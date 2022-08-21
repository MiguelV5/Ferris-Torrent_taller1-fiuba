use std::{
    collections::HashMap,
    error::Error,
    fmt,
    fs::File,
    io::{BufRead, BufReader},
};

const TORRENTS_PATH: &str = "torrents_path";
const NUMBER_THREADS: &str = "number_threads";
const WHITESPACE: &str = " ";

type ResultConfig<T> = Result<T, ConfigFileDataError>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ConfigFileData {
    pub number_of_threads: usize,
    pub torrents_path: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigFileDataError {
    FileNotFound,
    BadSize,
    ThreadsNotANumber,
    MissingThreads,
    BadLine,
    InvalidFormat,
    MissingPath(String),
}

impl fmt::Display for ConfigFileDataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error del archivo de configuración.\n Backtrace: {:?}\n",
            self
        )
    }
}

impl Error for ConfigFileDataError {}

impl ConfigFileData {
    /// Datos del archivo de configuración:
    /// Deben de ingresarse en el formato "clave valor"
    /// Con un espacio de separador.
    /// Requiere las claves:
    /// number_of_threads: número de threads que se abrirán en la threadpool
    /// torrents_path: path del directorio de torrents disponibles para el tracker
    /// Por ejemplo:
    /// ```txt
    /// number_of_threads <nro_puerto>
    /// torrents_path <path_descargas>
    /// ```
    ///
    pub fn new(config_file_path: &str) -> Result<ConfigFileData, ConfigFileDataError> {
        let lines = read_config_file(config_file_path)?;
        if lines.len() != 2 {
            return Err(ConfigFileDataError::BadSize);
        }
        let config_map = get_data_from_config_file(lines)?;
        Ok(ConfigFileData {
            number_of_threads: read_number_of_threads(&config_map)?,
            torrents_path: read_path(&config_map, TORRENTS_PATH)?,
        })
    }

    ///Number of thread getter
    pub fn get_number_of_threads(&self) -> usize {
        self.number_of_threads
    }

    ///Torrents path getter
    pub fn get_torrents_path(&self) -> String {
        self.torrents_path.clone()
    }
}

/// Se encarga de extraer directamente la info del archivo de configuración
/// y se coloca en un hashmap, que será utilizado luego para mapear
/// los datos
///
fn get_data_from_config_file(
    lines: Vec<String>,
) -> Result<HashMap<String, String>, ConfigFileDataError> {
    let mut config_map: HashMap<String, String> = HashMap::new();
    for line in lines {
        let pair_data_value: Vec<String> = line
            .split(WHITESPACE)
            .map(|s| s.trim().to_string())
            .collect();

        match &pair_data_value[..] {
            [key, value] => {
                config_map.insert(key.to_string(), value.to_string());
            }
            _ => return Err(ConfigFileDataError::InvalidFormat),
        }
    }
    Ok(config_map)
}

fn read_number_of_threads(
    config_map: &HashMap<String, String>,
) -> Result<usize, ConfigFileDataError> {
    if let Some(value_read) = config_map.get(NUMBER_THREADS) {
        match value_read.parse::<usize>() {
            Ok(port_read) => return Ok(port_read),
            Err(_) => return Err(ConfigFileDataError::ThreadsNotANumber),
        };
    }
    Err(ConfigFileDataError::MissingThreads)
}

fn read_path(
    config_map: &HashMap<String, String>,
    torrents_path: &str,
) -> Result<String, ConfigFileDataError> {
    if let Some(value_read) = config_map.get(torrents_path) {
        return Ok(value_read.clone());
    }
    Err(ConfigFileDataError::MissingPath(torrents_path.to_string()))
}

/// Se encarga de leer la información de configuración
/// Devuelve un vector de Strings en el que cada elemento es una línea del archivo leído
///
fn read_config_file(filename: &str) -> ResultConfig<Vec<String>> {
    let file = match File::open(filename) {
        Ok(file_open) => file_open,
        Err(_) => return Err(ConfigFileDataError::FileNotFound),
    };

    let buf = BufReader::new(file);
    let result = buf
        .lines()
        .map(|l| l.expect("Could not parse line"))
        .collect();
    Ok(result)
}

#[cfg(test)]
mod tests_config_file {

    use super::*;

    #[test]
    fn read_config_file_ok() -> Result<(), ConfigFileDataError> {
        let file_dir = "config.txt";
        let config_data = match read_config_file(file_dir) {
            Ok(config_data) => config_data,
            Err(error) => return Err(error),
        };
        assert_eq!(config_data.len(), 2);
        Ok(())
    }

    #[test]
    fn read_fill_config_data_ok() -> Result<(), ConfigFileDataError> {
        let file_dir = "config.txt";
        let config = match ConfigFileData::new(file_dir) {
            Ok(config) => config,
            Err(error) => return Err(error),
        };
        assert_eq!(config.number_of_threads, 4);
        assert_eq!(config.torrents_path, "ferris_tracker/torrents_for_test");
        Ok(())
    }
}
