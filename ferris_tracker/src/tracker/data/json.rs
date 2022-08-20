extern crate chrono;

use std::{error::Error, fmt, fs};

use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::tracker::data::constants::{COMPLETED, CONNECTIONS, TIMES};

#[derive(Serialize, Deserialize)]
struct Json {
    times: Vec<String>,
    connections: Vec<u32>,
    completed: Vec<u32>,
}

#[derive(PartialEq, Eq, Debug)]
pub enum JsonError {
    OpeningFile,
    Format,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for JsonError {}

impl Json {
    pub fn new() -> Self {
        Json {
            times: vec![],
            connections: vec![],
            completed: vec![],
        }
    }
    pub fn new_from_file(file_location: &str) -> Result<Self, JsonError> {
        let data = match fs::read_to_string(file_location) {
            Ok(string_file) => string_file,
            Err(_) => return Err(JsonError::OpeningFile),
        };
        let json_file: Json = match serde_json::from_str(&data) {
            Ok(json_struct) => json_struct,
            Err(_) => return Err(JsonError::Format),
        };

        Ok(Json {
            times: json_file.times,
            connections: json_file.connections,
            completed: json_file.completed,
        })
    }
    pub fn add_connections(&mut self, is_completed: bool) {
        let date = Local::now();
        let date_now_str = date.format("%Y-%m-%d %H:%M:%S").to_string();
        self.times.push(date_now_str);
        match self.connections.last() {
            Some(last_num) => {
                let new = last_num.to_owned() + 1;
                self.connections.push(new);
            }
            None => self.connections.push(1),
        }
        if is_completed {
            match self.completed.last() {
                Some(last_num) => {
                    let new = last_num.to_owned() + 1;
                    self.completed.push(new);
                }
                None => self.completed.push(1),
            }
        }
    }
    pub fn get_json_string(&self) -> String {
        let json_struct = json!({
            TIMES: self.times,
            CONNECTIONS: self.connections,
            COMPLETED: self.completed,
        });

        json_struct.to_string()
    }
}
