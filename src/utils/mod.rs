use std::path::PathBuf;

use tokio::fs;

use crate::{config::Config, error::AppResult};

pub async fn ensure_data_dirs(config: &Config) -> AppResult<()> {
    fs::create_dir_all(&config.data_dir).await?;
    fs::create_dir_all(upload_dir(config)).await?;
    fs::create_dir_all(preview_dir(config)).await?;
    fs::create_dir_all(tmp_dir(config)).await?;
    Ok(())
}

pub fn upload_dir(config: &Config) -> PathBuf {
    config.data_dir.join("uploads")
}

pub fn preview_dir(config: &Config) -> PathBuf {
    config.data_dir.join("previews")
}

pub fn tmp_dir(config: &Config) -> PathBuf {
    config.data_dir.join("tmp")
}

pub fn sanitize_filename(input: &str) -> String {
    let sanitized: String = input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ' ') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches(['.', ' ']).trim().to_string();
    if sanitized.is_empty() {
        "upload.bin".to_string()
    } else {
        sanitized
    }
}
