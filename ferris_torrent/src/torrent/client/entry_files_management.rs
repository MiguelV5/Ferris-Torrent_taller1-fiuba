use std::{
    env,
    error::Error,
    fmt,
    fs::{self, ReadDir},
    path::Path,
};

use log::error;

#[derive(Debug)]
pub enum EntryFilesError {
    NoArgs,
    NotFound(String),
    Folder(String),
}

impl fmt::Display for EntryFilesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for EntryFilesError {}

fn add_files_from_folder(list: &mut Vec<String>, folder: ReadDir) -> Result<(), EntryFilesError> {
    for file in folder {
        match file {
            Ok(file_ok) => {
                let str_file = file_ok.path().display().to_string();
                list.push(str_file);
            }
            Err(error) => return Err(EntryFilesError::Folder(error.to_string())),
        }
    }
    Ok(())
}

///
/// Funcion encargada de analizar la ruta enviada por consola y generar una lista con todos los archivos .torrent que
/// pertenecen a ese directorio o archivo.
/// En caso de ser un directorio, la lista posee cada una de las rutas a cada archivo . torrent.
/// En caso de ser un archivo, tambien se lo coloca en una lista y se lo retorna como único elemento de la lista.
///
pub fn create_list_files() -> Result<Vec<String>, EntryFilesError> {
    let mut list_files = vec![];
    let mut iter_args = env::args();
    iter_args.next();

    for args in iter_args {
        let path_args = Path::new(&args);
        if path_args.is_file() {
            list_files.push(args)
        } else if path_args.is_dir() {
            match fs::read_dir(args) {
                Ok(folder) => add_files_from_folder(&mut list_files, folder)?,
                Err(error) => return Err(EntryFilesError::Folder(error.to_string())),
            }
        } else {
            error!("No se encontro el archivo o carpeta ingresado");
            return Err(EntryFilesError::NotFound(args));
        }
    }
    if list_files.is_empty() {
        error!("No ingreso archivo/s o carpeta/s por terminal");
        return Err(EntryFilesError::NoArgs);
    }
    Ok(list_files)
}
