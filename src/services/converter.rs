use std::path::Path;

use tokio::{
    fs,
    time::{timeout, Duration},
};

use crate::{
    config::Config,
    error::{AppError, AppResult},
};

pub async fn prepare_preview(config: &Config, source: &Path, preview_pdf: &Path) -> AppResult<i64> {
    if let Some(parent) = preview_pdf.parent() {
        fs::create_dir_all(parent).await?;
    }

    if is_pdf(source) {
        fs::copy(source, preview_pdf).await?;
        return count_pdf_pages(preview_pdf);
    }

    ensure_supported(source)?;
    if config.converter.office_command.trim().is_empty() {
        return Err(AppError::External(
            "document conversion is not configured; set converter.office_command".to_string(),
        ));
    }
    run_external_converter(config, source, preview_pdf).await?;
    count_pdf_pages(preview_pdf)
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

fn count_pdf_pages(path: &Path) -> AppResult<i64> {
    let document = lopdf::Document::load(path)
        .map_err(|error| AppError::BadRequest(format!("invalid PDF file: {error}")))?;
    let pages = document.get_pages().len() as i64;
    if pages == 0 {
        Err(AppError::BadRequest("PDF file has no pages".to_string()))
    } else {
        Ok(pages)
    }
}

fn ensure_supported(path: &Path) -> AppResult<()> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        extension.as_str(),
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "jpg" | "jpeg" | "png" | "bmp" | "txt"
    ) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "unsupported file type: .{extension}"
        )))
    }
}

async fn run_external_converter(config: &Config, source: &Path, output: &Path) -> AppResult<()> {
    let input = source
        .to_str()
        .ok_or_else(|| AppError::External("source path is not valid UTF-8".to_string()))?;
    let output_path = output
        .to_str()
        .ok_or_else(|| AppError::External("preview path is not valid UTF-8".to_string()))?;
    let command_line = config
        .converter
        .office_command
        .replace("{input}", input)
        .replace("{output}", output_path);

    let mut child = if cfg!(windows) {
        let mut command = tokio::process::Command::new("cmd");
        command.arg("/C").arg(&command_line).kill_on_drop(true);
        command
    } else {
        let mut command = tokio::process::Command::new("sh");
        command.arg("-c").arg(&command_line).kill_on_drop(true);
        command
    };
    let status = timeout(
        Duration::from_secs(config.converter.command_timeout_seconds.max(5)),
        child.status(),
    )
    .await
    .map_err(|_| AppError::External("document converter timed out".to_string()))??;

    if status.success() && output.exists() {
        Ok(())
    } else {
        Err(AppError::External(format!(
            "document converter failed with status {status}"
        )))
    }
}
