use std::path::Path;

use calamine::{open_workbook_auto, Reader};

use crate::error::{AppError, AppResult};

const SUPPORTED_EXTENSIONS: &[&str] = &["xlsx", "xls", "xlsm", "csv", "txt"];

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
            "不支持该用户导入文件格式，仅支持 XLSX、XLS、XLSM、CSV 和 TXT".to_string(),
        ))
    }
}

pub fn parse_student_ids(path: &Path, bytes: &[u8]) -> AppResult<Vec<String>> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let ids = match extension.as_str() {
        "xlsx" | "xls" | "xlsm" => parse_excel(path)?,
        _ => parse_plain(bytes),
    };

    let mut unique = Vec::new();
    for id in ids {
        let id = id.trim().to_string();
        if id.is_empty() || unique.iter().any(|existing| existing == &id) {
            continue;
        }
        unique.push(id);
    }

    if unique.is_empty() {
        return Err(AppError::BadRequest(
            "import file does not contain any student id".to_string(),
        ));
    }

    Ok(unique)
}

#[cfg(test)]
mod tests {
    use super::ensure_supported_file_name;

    #[test]
    fn import_file_name_rejects_unknown_extensions() {
        assert!(ensure_supported_file_name("users.xlsx").is_ok());
        assert!(ensure_supported_file_name("users.CSV").is_ok());
        assert!(ensure_supported_file_name("users.exe").is_err());
        assert!(ensure_supported_file_name("users").is_err());
    }
}

fn parse_excel(path: &Path) -> AppResult<Vec<String>> {
    let mut workbook =
        open_workbook_auto(path).map_err(|error| AppError::BadRequest(error.to_string()))?;
    let Some(range) = workbook.worksheet_range_at(0) else {
        return Ok(Vec::new());
    };
    let range = range.map_err(|error| AppError::BadRequest(error.to_string()))?;

    Ok(range
        .rows()
        .filter_map(|row| row.first())
        .map(|cell| cell.to_string())
        .collect())
}

fn parse_plain(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter_map(|line| line.split([',', '\t', ';']).next())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
