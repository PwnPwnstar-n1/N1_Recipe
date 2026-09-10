use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, Runtime};

const LOG_NAME: &str = "n1-recipe.log";
const PREVIOUS_LOG_NAME: &str = "n1-recipe.previous.log";
const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

pub fn log_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|directory| directory.join(LOG_NAME))
        .map_err(|error| format!("diagnostic log path unavailable: {error}"))
}

pub fn init<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let path = log_path(app)?;
    let directory = path
        .parent()
        .ok_or_else(|| "diagnostic log directory unavailable".to_string())?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("create diagnostic directory: {error}"))?;
    if path.metadata().map(|meta| meta.len()).unwrap_or(0) > MAX_LOG_BYTES {
        let previous = directory.join(PREVIOUS_LOG_NAME);
        if previous.exists() {
            fs::remove_file(&previous)
                .map_err(|error| format!("remove previous diagnostic log: {error}"))?;
        }
        fs::rename(&path, previous).map_err(|error| format!("rotate diagnostic log: {error}"))?;
    }
    Ok(path)
}

/// Append a privacy-safe lifecycle event. Callers must never include recipe
/// content, template values, clipboard data, file paths, or user search text.
pub fn log<R: Runtime>(app: &AppHandle<R>, event: &str) {
    let result = (|| -> Result<(), String> {
        let path = init(app)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| format!("open diagnostic log: {error}"))?;
        writeln!(
            file,
            "{} {}",
            Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z"),
            event
        )
        .map_err(|error| format!("write diagnostic log: {error}"))
    })();
    eprintln!("N1_Recipe {event}");
    if let Err(error) = result {
        eprintln!("N1_Recipe diagnostics error: {error}");
    }
}
