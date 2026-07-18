use std::{path::Path, process::Stdio};

use tokio::{
    fs, task,
    time::{timeout, Duration},
};
use tracing::info;

use crate::{
    config::Config,
    error::{AppError, AppResult},
};

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "jpg", "jpeg", "png", "bmp", "txt",
];

pub fn ensure_supported_file_name(file_name: &str) -> AppResult<()> {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "不支持该文件格式，仅支持 PDF、Word、Excel、PPT、JPG、PNG、BMP 和 TXT".to_string(),
        ))
    }
}

pub async fn prepare_preview(config: &Config, source: &Path, preview_pdf: &Path) -> AppResult<i64> {
    if let Some(parent) = preview_pdf.parent() {
        fs::create_dir_all(parent).await?;
    }

    if is_pdf(source) {
        info!(source = %source.display(), preview = %preview_pdf.display(), "copying uploaded PDF for preview");
        fs::copy(source, preview_pdf).await?;
        return count_pdf_pages_async(config, preview_pdf).await;
    }

    ensure_supported(source)?;
    info!(source = %source.display(), preview = %preview_pdf.display(), "running document converter");
    run_external_converter(config, source, preview_pdf).await?;
    count_pdf_pages_async(config, preview_pdf).await
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

async fn count_pdf_pages_async(config: &Config, path: &Path) -> AppResult<i64> {
    let path = path.to_path_buf();
    timeout(
        Duration::from_secs(config.converter.command_timeout_seconds.max(5)),
        task::spawn_blocking(move || count_pdf_pages(&path)),
    )
    .await
    .map_err(|_| AppError::External("PDF inspection timed out".to_string()))?
    .map_err(|error| AppError::External(format!("PDF inspection task failed: {error}")))?
}

fn ensure_supported(path: &Path) -> AppResult<()> {
    ensure_supported_file_name(path.to_string_lossy().as_ref())
}

async fn run_external_converter(config: &Config, source: &Path, output: &Path) -> AppResult<()> {
    let input = source
        .to_str()
        .ok_or_else(|| AppError::External("source path is not valid UTF-8".to_string()))?;
    let output_path = output
        .to_str()
        .ok_or_else(|| AppError::External("preview path is not valid UTF-8".to_string()))?;
    let output_dir = output
        .parent()
        .and_then(Path::to_str)
        .ok_or_else(|| AppError::External("preview directory is not valid UTF-8".to_string()))?;
    let program = converter_program(config);
    let default_args = [
        "--headless",
        "--convert-to",
        "pdf",
        "--outdir",
        "{output_dir}",
        "{input}",
    ];
    let arguments: &[String] = &config.converter.office_args;
    let mut command = tokio::process::Command::new(program);
    let templates: Vec<&str> = if arguments.is_empty() {
        default_args.to_vec()
    } else {
        arguments.iter().map(String::as_str).collect()
    };
    command.args(
        templates
            .iter()
            .map(|argument| replace_placeholders(argument, input, output_path, output_dir)),
    );
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

    if result.status.success() && !output.exists() {
        let generated = output
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(source.file_stem().unwrap_or_default())
            .with_extension("pdf");
        if generated.exists() {
            fs::rename(generated, output).await?;
        }
    }
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

fn converter_program(config: &Config) -> String {
    if !config.converter.office_program.trim().is_empty() {
        return config.converter.office_program.clone();
    }
    #[cfg(windows)]
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            for relative in [
                "tools/LibreOffice/program/soffice.exe",
                "tools/LibreOfficePortable/App/libreoffice/program/soffice.exe",
            ] {
                let candidate = directory.join(relative);
                if candidate.is_file() {
                    return candidate.to_string_lossy().into_owned();
                }
            }
        }
    }
    if cfg!(windows) {
        "soffice.exe".into()
    } else {
        "libreoffice".into()
    }
}

fn replace_placeholders(template: &str, input: &str, output: &str, output_dir: &str) -> String {
    template
        .replace("{input}", input)
        .replace("{output}", output)
        .replace("{output_dir}", output_dir)
}

#[cfg(test)]
mod tests {
    use super::{ensure_supported_file_name, replace_placeholders};

    #[test]
    fn placeholders_preserve_spaces_as_part_of_one_argument() {
        let input = r"C:\Users\ACM User\input file.docx";
        let output = r"C:\Printing Platform\preview file.pdf";
        assert_eq!(
            replace_placeholders("{input}", input, output, "C:\\out"),
            input
        );
        assert_eq!(
            replace_placeholders("{output}", input, output, "C:\\out"),
            output
        );
    }

    #[test]
    fn upload_file_name_rejects_unknown_extensions() {
        assert!(ensure_supported_file_name("document.pdf").is_ok());
        assert!(ensure_supported_file_name("photo.JPEG").is_ok());
        assert!(ensure_supported_file_name("archive.zip").is_err());
        assert!(ensure_supported_file_name("no-extension").is_err());
    }
}
