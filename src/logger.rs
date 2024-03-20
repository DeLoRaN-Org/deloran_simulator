use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs::File, sync::Mutex};
use std::io::Write;

pub struct Logger {
    log_file: Mutex<File>,
    active_logger: bool,
    logger_println: bool,
}

impl Logger {
    pub fn new(path: &str, active_logger: bool, logger_println: bool) -> Self {
        let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .expect("Failed to open file");
        Self {
            log_file: Mutex::new(file),
            active_logger,
            logger_println
        }
    }

    pub fn now() -> u128 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
    }

    pub fn write(&self, content: &str) {
        if self.active_logger {
            writeln!(self.log_file.lock().unwrap(), "{}", content).expect("Error while logging to file");
        }
        if self.logger_println {
            println!("{}", content)
        }
    }
}