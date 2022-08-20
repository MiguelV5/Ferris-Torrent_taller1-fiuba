extern crate chrono;

use std::{error::Error, fmt, fs};

use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::tracker::data::constants::{COMPLETED, CONNECTIONS, TIMES, TORRENTS};

pub const ONE: u32 = 1;
pub const ZERO: u32 = 0;

#[derive(Serialize, Deserialize)]
struct Json {
    torrents: u32,
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
    pub fn new(torrents: u32) -> Self {
        Json {
            torrents,
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
            torrents: json_file.torrents,
            times: json_file.times,
            connections: json_file.connections,
            completed: json_file.completed,
        })
    }
    pub fn add_new_connection(&mut self, is_completed: bool) {
        let date = Local::now();
        let date_now_str = date.format("%Y-%m-%d %H:%M:%S").to_string();
        self.times.push(date_now_str);

        match self.connections.last() {
            Some(last_num) => {
                let new = last_num.to_owned() + ONE;
                self.connections.push(new);
            }
            None => self.connections.push(ONE),
        }

        match self.completed.last() {
            Some(last_num) => {
                let mut new_value = last_num.to_owned();
                if is_completed {
                    new_value += ONE;
                }
                self.completed.push(new_value);
            }
            None => {
                if is_completed {
                    self.completed.push(ONE)
                } else {
                    self.completed.push(ZERO)
                }
            }
        }
    }
    pub fn get_json_string(&self) -> String {
        let json_struct = json!({
            TORRENTS: self.torrents,
            TIMES: self.times,
            CONNECTIONS: self.connections,
            COMPLETED: self.completed,
        });

        json_struct.to_string()
    }
}

#[cfg(test)]
mod tests_json {
    use super::*;

    #[test]
    fn test_new_json() {
        let torrents_expected = 3;
        let json = Json::new(torrents_expected);
        assert_eq!(json.torrents, torrents_expected);
        assert!(json.times.is_empty());
        assert!(json.connections.is_empty());
        assert!(json.completed.is_empty());
    }

    #[test]
    fn test_new_from_files_json() {
        let torrents_expected = 3;
        let times_expected = vec![
            "2022-08-19 12:30:00".to_owned(),
            "2022-08-19 13:50:00".to_owned(),
            "2022-08-19 14:30:00".to_owned(),
        ];
        let connections_expected = vec![1, 2, 5];
        let completed_expected = vec![0, 0, 1];
        let json_file = Json::new_from_file("test.json");
        assert!(json_file.is_ok());
        if let Ok(json) = json_file {
            assert_eq!(json.torrents, torrents_expected);
            assert_eq!(json.times, times_expected);
            assert_eq!(json.connections, connections_expected);
            assert_eq!(json.completed, completed_expected);
        }
    }

    #[test]
    fn test_new_connections_json() {
        let mut json = Json::new(2);
        let connections_expected = vec![1, 2, 3, 4, 5];
        let completed_expected = vec![0, 0, 1, 2, 2];

        json.add_new_connection(false);
        json.add_new_connection(false);
        json.add_new_connection(true);
        json.add_new_connection(true);
        json.add_new_connection(false);

        assert_eq!(json.connections, connections_expected);
        assert_eq!(json.completed, completed_expected);
    }

    #[test]
    fn test_to_string_json() {
        let mut json = Json::new(2);
        let connections_expected = vec![1, 2, 3];
        let completed_expected = vec![0, 0, 1];

        json.add_new_connection(false);
        json.add_new_connection(false);
        json.add_new_connection(true);

        let json_string = json.get_json_string();
        let json_struct: Json = match serde_json::from_str(&json_string) {
            Ok(json) => json,
            Err(_) => panic!("Can't transform to json"),
        };
        assert_eq!(json_struct.connections, connections_expected);
        assert_eq!(json_struct.completed, completed_expected);
    }
}
