use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    /// Runtime state. Prefer an absolute path in production. When omitted the
    /// platform chooses a persistent, per-machine/per-user OS data directory.
    pub data_dir: PathBuf,
    pub initial_admin_student_id: String,
    pub min_password_length: usize,
    pub default_daily_limit: i64,
    pub session_days: i64,
    pub queue_poll_seconds: u64,
    pub cleanup_interval_hours: u64,
    pub file_retention_days: i64,
    pub temp_upload_retention_hours: i64,
    pub limits: LimitsConfig,
    pub printer: PrinterConfig,
    pub converter: ConverterConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub bind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    pub max_upload_bytes: u64,
    pub max_import_bytes: u64,
    pub max_files_per_request: usize,
    pub max_pages_per_file: i64,
    pub max_temp_bytes_per_user: u64,
    pub conversion_concurrency: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PrinterConfig {
    pub name: String,
    pub simulate: bool,
    pub command_timeout_seconds: u64,
    pub job_discovery_seconds: u64,
    pub pdf_printer_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ConverterConfig {
    pub office_program: String,
    pub office_args: Vec<String>,
    pub command_timeout_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            data_dir: default_data_dir(),
            initial_admin_student_id: "admin".into(),
            min_password_length: 8,
            default_daily_limit: 10,
            session_days: 30,
            queue_poll_seconds: 5,
            cleanup_interval_hours: 6,
            file_retention_days: 365,
            temp_upload_retention_hours: 24,
            limits: LimitsConfig::default(),
            printer: PrinterConfig::default(),
            converter: ConverterConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:80".into(),
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_upload_bytes: 50 * 1024 * 1024,
            max_import_bytes: 2 * 1024 * 1024,
            max_files_per_request: 10,
            max_pages_per_file: 500,
            max_temp_bytes_per_user: 200 * 1024 * 1024,
            conversion_concurrency: 1,
        }
    }
}

impl Default for PrinterConfig {
    fn default() -> Self {
        Self {
            name: "HP LaserJet Professional P1106".into(),
            simulate: false,
            command_timeout_seconds: 60,
            job_discovery_seconds: 20,
            pdf_printer_path: String::new(),
        }
    }
}

impl Default for ConverterConfig {
    fn default() -> Self {
        Self {
            office_program: String::new(),
            office_args: Vec::new(),
            command_timeout_seconds: 180,
        }
    }
}

impl Config {
    pub fn load() -> AppResult<Self> {
        let path = config_path();
        let mut config = match fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error.into()),
        };
        if let Some(data_dir) = env::var_os("PRINTING_PLATFORM_DATA_DIR") {
            config.data_dir = PathBuf::from(data_dir);
        }
        config.validate()?;
        Ok(config)
    }

    pub fn database_url(&self) -> String {
        format!(
            "sqlite://{}",
            self.data_dir
                .join("printing_platform.db")
                .to_string_lossy()
                .replace('\\', "/")
        )
    }

    fn validate(&self) -> AppResult<()> {
        if self.initial_admin_student_id.trim().is_empty() {
            return Err(AppError::BadRequest(
                "initial_admin_student_id cannot be empty".into(),
            ));
        }
        if self.min_password_length == 0 {
            return Err(AppError::BadRequest(
                "min_password_length must be greater than 0".into(),
            ));
        }
        if self.default_daily_limit < 0 {
            return Err(AppError::BadRequest(
                "default_daily_limit cannot be negative".into(),
            ));
        }
        if self.session_days < 1 || self.session_days > 365 {
            return Err(AppError::BadRequest(
                "session_days must be between 1 and 365".into(),
            ));
        }
        if self.limits.max_upload_bytes == 0
            || self.limits.max_import_bytes == 0
            || self.limits.max_files_per_request == 0
            || self.limits.max_pages_per_file == 0
            || self.limits.max_temp_bytes_per_user < self.limits.max_upload_bytes
            || self.limits.conversion_concurrency == 0
        {
            return Err(AppError::BadRequest(
                "resource limits must be positive and max_temp_bytes_per_user must cover one upload"
                    .into(),
            ));
        }
        if !self.printer.simulate && self.printer.name.trim().is_empty() {
            return Err(AppError::BadRequest(
                "printer.name is required when simulation is disabled".into(),
            ));
        }
        Ok(())
    }
}

fn config_path() -> PathBuf {
    if let Some(path) = env::var_os("PRINTING_PLATFORM_CONFIG") {
        return PathBuf::from(path);
    }
    let sibling = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join("config.toml")));
    sibling
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from("config.toml"))
}

fn default_data_dir() -> PathBuf {
    if let Some(path) = env::var_os("PRINTING_PLATFORM_DATA_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(windows)]
    if let Some(path) = env::var_os("PROGRAMDATA") {
        return Path::new(&path).join("PrintingPlatform");
    }
    #[cfg(not(windows))]
    {
        if let Some(path) = env::var_os("XDG_DATA_HOME") {
            return Path::new(&path).join("printing-platform");
        }
        if let Some(home) = env::var_os("HOME") {
            return Path::new(&home).join(".local/share/printing-platform");
        }
        PathBuf::from("/var/lib/printing-platform")
    }
    #[cfg(windows)]
    PathBuf::from(r"C:\ProgramData\PrintingPlatform")
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn repository_config_is_deployment_ready() {
        let config: Config = toml::from_str(include_str!("../config.toml")).unwrap();
        config.validate().unwrap();
        assert_eq!(config.server.bind, "0.0.0.0:80");
        assert_eq!(config.printer.name, "HP LaserJet Professional P1106");
        assert!(!config.printer.simulate);
        assert_eq!(config.min_password_length, 8);
        assert_eq!(config.default_daily_limit, 10);
        assert!(config.limits.max_upload_bytes > 0);
    }

    #[test]
    fn password_length_is_configurable() {
        let config: Config = toml::from_str("min_password_length = 12").unwrap();
        config.validate().unwrap();
        assert_eq!(config.min_password_length, 12);
    }

    #[test]
    fn password_length_must_be_positive() {
        let config: Config = toml::from_str("min_password_length = 0").unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn default_daily_limit_is_configurable_and_non_negative() {
        let config: Config = toml::from_str("default_daily_limit = 25").unwrap();
        config.validate().unwrap();
        assert_eq!(config.default_daily_limit, 25);

        let invalid: Config = toml::from_str("default_daily_limit = -1").unwrap();
        assert!(invalid.validate().is_err());
    }
}
