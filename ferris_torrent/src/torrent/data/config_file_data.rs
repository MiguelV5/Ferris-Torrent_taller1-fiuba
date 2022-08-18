use std::{
    collections::HashMap,
    error::Error,
    fmt,
    fs::File,
    io::{BufRead, BufReader},
};

const PORT: &str = "port";
const DOWNLOAD: &str = "download";
const LOGS: &str = "logs";
const WHITESPACE: &str = " ";

type ResultConfig<T> = Result<T, ConfigFileDataError>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ConfigFileData {
    pub port: u32,
    pub log_path: String,
    pub download_path: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigFileDataError {
    FileNotFound,
    BadSize,
    PortNotANumber,
    BadLine,
    InvalidFormat,
    MissingPort,
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
    /// port: número de puerto en el que se escuharan conexiones
    /// download: path del directorio descarga del torrent
    /// logs: path del del directorio del archivo de logs
    /// Por ejemplo:
    /// ```txt
    /// port <nro_puerto>
    /// download <path_descargas>
    /// logs <path_logs>
    /// ```
    ///
    pub fn new(config_file_path: &str) -> Result<ConfigFileData, ConfigFileDataError> {
        let lines = read_config_file(config_file_path)?;
        if lines.len() != 3 {
            return Err(ConfigFileDataError::BadSize);
        }
        let config_map = get_data_from_config_file(lines)?;
        Ok(ConfigFileData {
            port: read_port(&config_map)?,
            log_path: read_path(&config_map, LOGS)?,
            download_path: read_path(&config_map, DOWNLOAD)?,
        })
    }

    ///Port getter
    pub fn get_port(&self) -> u32 {
        self.port
    }

    ///Log path getter
    pub fn get_log_path(&self) -> String {
        self.log_path.clone()
    }

    ///Download path getter
    pub fn get_download_path(&self) -> String {
        self.download_path.clone()
    }
}

/// Se encarga de extraer directamente la info del archivo de configuración
/// y se coloca en un hashmap, qque será utilizado luego para mapear
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

fn read_port(config_map: &HashMap<String, String>) -> Result<u32, ConfigFileDataError> {
    if let Some(value_read) = config_map.get(PORT) {
        match value_read.parse::<u32>() {
            Ok(port_read) => return Ok(port_read),
            Err(_) => return Err(ConfigFileDataError::PortNotANumber),
        };
    }
    Err(ConfigFileDataError::MissingPort)
}

fn read_path(
    config_map: &HashMap<String, String>,
    results_path: &str,
) -> Result<String, ConfigFileDataError> {
    if let Some(value_read) = config_map.get(results_path) {
        return Ok(value_read.clone());
    }
    Err(ConfigFileDataError::MissingPath(results_path.to_string()))
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
        assert_eq!(config_data.len(), 3);
        Ok(())
    }

    #[test]
    fn read_fill_config_data_ok() -> Result<(), ConfigFileDataError> {
        let file_dir = "config.txt";
        let config = match ConfigFileData::new(file_dir) {
            Ok(config) => config,
            Err(error) => return Err(error),
        };
        assert_eq!(config.port, 6881);
        assert_eq!(config.download_path, "ferris_torrent/results/download");
        assert_eq!(config.log_path, "ferris_torrent/results/logs");
        Ok(())
    }
}
