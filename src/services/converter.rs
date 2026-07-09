use std::path::Path;

use tokio::fs;

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
        return Ok(count_pdf_pages(preview_pdf).unwrap_or(1));
    }

    if !config.converter.office_command.trim().is_empty() {
        run_external_converter(&config.converter.office_command, source, preview_pdf).await?;
        return Ok(count_pdf_pages(preview_pdf).unwrap_or(1));
    }

    write_placeholder_pdf(preview_pdf, source).await?;
    Ok(1)
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

fn count_pdf_pages(path: &Path) -> Option<i64> {
    let document = lopdf::Document::load(path).ok()?;
    Some(document.get_pages().len().max(1) as i64)
}

async fn run_external_converter(template: &str, source: &Path, output: &Path) -> AppResult<()> {
    let input = source
        .to_str()
        .ok_or_else(|| AppError::External("source path is not valid UTF-8".to_string()))?;
    let output_path = output
        .to_str()
        .ok_or_else(|| AppError::External("preview path is not valid UTF-8".to_string()))?;
    let command = template
        .replace("{input}", input)
        .replace("{output}", output_path);

    let status = if cfg!(windows) {
        tokio::process::Command::new("cmd")
            .arg("/C")
            .arg(command)
            .status()
            .await?
    } else {
        tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .status()
            .await?
    };

    if status.success() && output.exists() {
        Ok(())
    } else {
        Err(AppError::External(format!(
            "document converter failed with status {status}"
        )))
    }
}

async fn write_placeholder_pdf(output: &Path, source: &Path) -> AppResult<()> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("uploaded file");
    let text = format!(
        "Preview placeholder for {name}. Configure converter.office_command for Office/Image conversion."
    );
    let escaped = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");
    let stream = format!("BT /F1 12 Tf 72 720 Td ({escaped}) Tj ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_string(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        format!("<< /Length {} >>\nstream\n{}\nendstream", stream.as_bytes().len(), stream),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.as_bytes().len());
        pdf.push_str(&format!("{} 0 obj\n{}\nendobj\n", index + 1, object));
    }
    let xref_start = pdf.as_bytes().len();
    pdf.push_str("xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer << /Root 1 0 R /Size 6 >>\nstartxref\n{xref_start}\n%%EOF\n"
    ));

    fs::write(output, pdf).await?;
    Ok(())
}
