use std::fs::OpenOptions;
use std::io::Write;

const LOG_PATH: &str = "termboxd.log";

pub fn log_error(message: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(LOG_PATH) {
        let _ = writeln!(file, "{message}");
    }
}
