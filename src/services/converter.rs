use std::{path::Path, process::Stdio};

use tokio::{
    fs, task,
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
        return count_pdf_pages_async(preview_pdf).await;
    }

    ensure_supported(source)?;
    if config.converter.office_program.trim().is_empty()
        && config.converter.office_command.trim().is_empty()
    {
        return Err(AppError::External(
            "document conversion is not configured; set converter.office_program and converter.office_args"
                .to_string(),
        ));
    }
    run_external_converter(config, source, preview_pdf).await?;
    count_pdf_pages_async(preview_pdf).await
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

async fn count_pdf_pages_async(path: &Path) -> AppResult<i64> {
    let path = path.to_path_buf();
    task::spawn_blocking(move || count_pdf_pages(&path))
        .await
        .map_err(|error| AppError::External(format!("PDF inspection task failed: {error}")))?
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
    let mut command = if !config.converter.office_program.trim().is_empty() {
        let mut command = tokio::process::Command::new(&config.converter.office_program);
        command.args(
            config
                .converter
                .office_args
                .iter()
                .map(|argument| replace_placeholders(argument, input, output_path)),
        );
        command
    } else {
        // Legacy command-string mode is retained for existing installations.
        // It is inherently sensitive to shell quoting, so new configurations
        // should always use office_program and office_args.
        let command_line =
            replace_placeholders(&config.converter.office_command, input, output_path);
        let mut command = if cfg!(windows) {
            let mut command = tokio::process::Command::new("cmd");
            command.arg("/C").arg(command_line);
            command
        } else {
            let mut command = tokio::process::Command::new("sh");
            command.arg("-c").arg(command_line);
            command
        };
        command.kill_on_drop(true);
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let result = timeout(
        Duration::from_secs(config.converter.command_timeout_seconds.max(5)),
        command.output(),
    )
    .await
    .map_err(|_| AppError::External("document converter timed out".to_string()))??;

    if result.status.success() && output.exists() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
        let details = if !stderr.is_empty() { stderr } else { stdout };
        Err(AppError::External(format!(
            "document converter failed with status {}{}",
            result.status,
            if details.is_empty() {
                String::new()
            } else {
                format!(": {details}")
            }
        )))
    }
}

fn replace_placeholders(template: &str, input: &str, output: &str) -> String {
    template
        .replace("{input}", input)
        .replace("{output}", output)
}

#[cfg(test)]
mod tests {
    use super::replace_placeholders;

    #[test]
    fn placeholders_preserve_spaces_as_part_of_one_argument() {
        let input = r"C:\Users\ACM User\input file.docx";
        let output = r"C:\Print Server\preview file.pdf";
        assert_eq!(replace_placeholders("{input}", input, output), input);
        assert_eq!(replace_placeholders("{output}", input, output), output);
    }
}
