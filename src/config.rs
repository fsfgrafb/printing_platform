use std::{fs, path::PathBuf};

use serde::Deserialize;

use crate::error::AppResult;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default = "default_database_url")]
    pub database_url: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    #[serde(default = "default_initial_admin")]
    pub initial_admin_student_id: String,
    #[serde(default = "default_session_days")]
    pub session_days: i64,
    #[serde(default = "default_queue_poll_seconds")]
    pub queue_poll_seconds: u64,
    #[serde(default = "default_cleanup_interval_hours")]
    pub cleanup_interval_hours: u64,
    #[serde(default = "default_file_retention_days")]
    pub file_retention_days: i64,
    #[serde(default = "default_temp_upload_retention_hours")]
    pub temp_upload_retention_hours: i64,
    #[serde(default)]
    pub printer: PrinterConfig,
    #[serde(default)]
    pub converter: ConverterConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrinterConfig {
    #[serde(default = "default_printer_name")]
    pub name: String,
    #[serde(default = "default_true")]
    pub simulate: bool,
    #[serde(default = "default_command_timeout")]
    pub command_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConverterConfig {
    #[serde(default)]
    pub office_command: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database_url: default_database_url(),
            data_dir: default_data_dir(),
            initial_admin_student_id: default_initial_admin(),
            session_days: default_session_days(),
            queue_poll_seconds: default_queue_poll_seconds(),
            cleanup_interval_hours: default_cleanup_interval_hours(),
            file_retention_days: default_file_retention_days(),
            temp_upload_retention_hours: default_temp_upload_retention_hours(),
            printer: PrinterConfig::default(),
            converter: ConverterConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
        }
    }
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self {
            name: default_printer_name(),
            simulate: true,
            command_timeout_seconds: default_command_timeout(),
        }
    }
}

impl Default for ConverterConfig {
    fn default() -> Self {
        Self {
            office_command: String::new(),
        }
    }
}

impl Config {
    pub fn load(path: &str) -> AppResult<Self> {
        match fs::read_to_string(path) {
            Ok(content) => Ok(toml::from_str(&content)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error.into()),
        }
    }
}

fn default_bind() -> String {
    "127.0.0.1:8080".to_string()
}

fn default_database_url() -> String {
    "sqlite://data/print-server.db".to_string()
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("data")
}

fn default_initial_admin() -> String {
    "admin".to_string()
}

fn default_session_days() -> i64 {
    365
}

fn default_queue_poll_seconds() -> u64 {
    5
}

fn default_cleanup_interval_hours() -> u64 {
    6
}

fn default_file_retention_days() -> i64 {
    365
}

fn default_temp_upload_retention_hours() -> i64 {
    24
}

fn default_printer_name() -> String {
    "HP LaserJet P1106".to_string()
}

fn default_true() -> bool {
    true
}

fn default_command_timeout() -> u64 {
    60
}
