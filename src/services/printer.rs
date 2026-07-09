use std::path::Path;

use tokio::time::{sleep, timeout, Duration};

use crate::{
    config::Config,
    error::{AppError, AppResult},
};

pub async fn print_pdf(config: &Config, pdf_path: &Path) -> AppResult<()> {
    if config.printer.simulate {
        sleep(Duration::from_secs(2)).await;
        return Ok(());
    }

    #[cfg(windows)]
    {
        let command = tokio::process::Command::new("cmd")
            .arg("/C")
            .arg("print")
            .arg(format!("/d:{}", config.printer.name))
            .arg(pdf_path)
            .status();

        let status = timeout(
            Duration::from_secs(config.printer.command_timeout_seconds),
            command,
        )
        .await
        .map_err(|_| AppError::External("print command timed out".to_string()))??;

        if status.success() {
            Ok(())
        } else {
            Err(AppError::External(format!(
                "print command failed with status {status}"
            )))
        }
    }

    #[cfg(not(windows))]
    {
        let _ = pdf_path;
        Err(AppError::External(
            "real printing is only implemented for Windows".to_string(),
        ))
    }
}

pub async fn status(config: &Config) -> String {
    if config.printer.simulate {
        "simulated".to_string()
    } else {
        "unknown".to_string()
    }
}
