use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs::File, sync::Mutex};
use std::io::Write;
pub struct Logger {
    log_file: Mutex<File>,
}

impl Logger {
    pub fn new(path: &str) -> Self {
        let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .expect("Failed to open file");
        Self {
            log_file: Mutex::new(file),
        }
    }

    pub fn now() -> u128 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
    }

    pub fn write(&self, content: &str) {
        writeln!(self.log_file.lock().unwrap(), "{},{}", Self::now(), content).expect("Error while logging to file");
    }
}