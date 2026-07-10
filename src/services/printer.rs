use std::{path::Path, process::Stdio};

use chrono::Local;
use serde::{Deserialize, Serialize};
use tokio::{
    process::Command,
    time::{timeout, Duration},
};

use crate::{
    config::Config,
    error::{AppError, AppResult},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PrinterJob {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub total_pages: i64,
    #[serde(default)]
    pub pages_printed: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PrinterState {
    pub mode: String,
    pub queue_name: String,
    pub available: bool,
    pub status: String,
    pub blocked: bool,
    pub blocking_reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub jobs: Vec<PrinterJob>,
    pub checked_at: String,
    pub error: Option<String>,
    pub toner_alert_acknowledged: bool,
}

impl PrinterState {
    pub fn starting() -> Self {
        Self {
            mode: "starting".into(),
            queue_name: String::new(),
            available: false,
            status: "starting".into(),
            blocked: true,
            blocking_reasons: vec!["打印机状态尚未初始化".into()],
            warnings: vec![],
            jobs: vec![],
            checked_at: Local::now().to_rfc3339(),
            error: None,
            toner_alert_acknowledged: false,
        }
    }
}

#[derive(Debug, Deserialize)]
struct BackendStatus {
    #[serde(default)]
    available: bool,
    #[serde(default)]
    status: String,
    #[serde(default)]
    is_offline: bool,
    #[serde(default)]
    is_paused: bool,
    #[serde(default)]
    is_in_error: bool,
    #[serde(default)]
    is_not_available: bool,
    #[serde(default)]
    need_user_intervention: bool,
    #[serde(default)]
    is_out_of_paper: bool,
    #[serde(default)]
    has_paper_problem: bool,
    #[serde(default)]
    is_paper_jammed: bool,
    #[serde(default)]
    is_toner_low: bool,
    #[serde(default)]
    is_out_of_memory: bool,
    #[serde(default)]
    is_output_bin_full: bool,
    #[serde(default)]
    is_door_opened: bool,
    #[serde(default)]
    is_manual_feed_required: bool,
    #[serde(default)]
    is_server_unknown: bool,
    #[serde(default)]
    jobs: Vec<PrinterJob>,
}

#[derive(Debug, Deserialize)]
pub struct SubmittedJob {
    pub job_id: i64,
    #[serde(default)]
    pub job_name: String,
}

pub async fn query_status(config: &Config) -> PrinterState {
    if config.printer.simulate {
        return PrinterState {
            mode: "simulated".into(),
            queue_name: config.printer.name.clone(),
            available: true,
            status: "Normal".into(),
            blocked: false,
            blocking_reasons: vec![],
            warnings: vec![],
            jobs: vec![],
            checked_at: Local::now().to_rfc3339(),
            error: None,
            toner_alert_acknowledged: false,
        };
    }

    match run_backend(config, &["-Action", "Status"])
        .await
        .and_then(|text| {
            serde_json::from_str::<BackendStatus>(&text).map_err(|error| {
                AppError::External(format!("invalid printer status response: {error}"))
            })
        }) {
        Ok(raw) => classify(config, raw),
        Err(error) => PrinterState {
            mode: "windows".into(),
            queue_name: config.printer.name.clone(),
            available: false,
            status: "Unavailable".into(),
            blocked: true,
            blocking_reasons: vec!["无法读取打印机状态".into()],
            warnings: vec![],
            jobs: vec![],
            checked_at: Local::now().to_rfc3339(),
            error: Some(error.to_string()),
            toner_alert_acknowledged: false,
        },
    }
}

fn classify(config: &Config, raw: BackendStatus) -> PrinterState {
    let mut reasons = Vec::new();
    if !raw.available || raw.is_not_available || raw.is_server_unknown {
        reasons.push("打印机不可用".into());
    }
    if raw.is_offline {
        reasons.push("打印机脱机".into());
    }
    if raw.is_paused {
        reasons.push("Windows 打印队列已暂停".into());
    }
    if raw.is_out_of_paper || raw.has_paper_problem {
        reasons.push("打印机缺纸或纸张异常".into());
    }
    if raw.is_paper_jammed {
        reasons.push("打印机卡纸".into());
    }
    if raw.is_door_opened {
        reasons.push("打印机舱门打开".into());
    }
    if raw.is_manual_feed_required {
        reasons.push("打印机要求手动进纸".into());
    }
    if raw.is_out_of_memory {
        reasons.push("打印机内存不足".into());
    }
    if raw.is_output_bin_full {
        reasons.push("打印机出纸槽已满".into());
    }
    if raw.is_in_error && reasons.is_empty() {
        reasons.push("打印机报告错误".into());
    }
    if raw.need_user_intervention && reasons.is_empty() {
        reasons.push("打印机需要人工处理".into());
    }
    if raw.jobs.iter().any(|job| {
        let status = job.status.to_ascii_lowercase();
        status.contains("error") || status.contains("blocked") || status.contains("offline")
    }) && reasons.is_empty()
    {
        reasons.push("Windows 打印作业报告错误".into());
    }

    let status_lower = raw.status.to_ascii_lowercase();
    let toner_low = raw.is_toner_low
        || status_lower.contains("tonerlow")
        || status_lower.contains("no toner")
        || status_lower.contains("notoner");
    let warnings = if toner_low {
        vec!["打印机可能墨粉不足，请管理员检查硒鼓（驱动估算结果）".into()]
    } else {
        vec![]
    };

    PrinterState {
        mode: "windows".into(),
        queue_name: config.printer.name.clone(),
        available: raw.available,
        status: if raw.status.is_empty() {
            "Unknown".into()
        } else {
            raw.status
        },
        blocked: !reasons.is_empty(),
        blocking_reasons: reasons,
        warnings,
        jobs: raw.jobs,
        checked_at: Local::now().to_rfc3339(),
        error: None,
        toner_alert_acknowledged: false,
    }
}

pub async fn submit_pdf(
    config: &Config,
    pdf_path: &Path,
    task_id: i64,
) -> AppResult<Option<SubmittedJob>> {
    if config.printer.simulate {
        tokio::time::sleep(Duration::from_secs(1)).await;
        return Ok(None);
    }
    let path = pdf_path
        .to_str()
        .ok_or_else(|| AppError::External("PDF path is not valid UTF-8".into()))?;
    let task = task_id.to_string();
    let discovery = config.printer.job_discovery_seconds.to_string();
    let text = run_backend(
        config,
        &[
            "-Action",
            "Submit",
            "-PdfPath",
            path,
            "-TaskId",
            &task,
            "-DiscoverySeconds",
            &discovery,
            "-PdfPrinterPath",
            &config.printer.pdf_printer_path,
        ],
    )
    .await?;
    let job = serde_json::from_str(&text).map_err(|error| {
        AppError::External(format!("invalid printer submission response: {error}"))
    })?;
    Ok(Some(job))
}

pub async fn cancel_job(config: &Config, job_id: i64) -> AppResult<()> {
    if config.printer.simulate {
        return Ok(());
    }
    let job_id = job_id.to_string();
    run_backend(config, &["-Action", "Cancel", "-JobId", &job_id]).await?;
    Ok(())
}

async fn run_backend(config: &Config, args: &[&str]) -> AppResult<String> {
    #[cfg(windows)]
    {
        let mut command = Command::new("powershell");
        command
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&config.printer.backend_script)
            .args(args)
            .arg("-PrinterName")
            .arg(&config.printer.name)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = timeout(
            Duration::from_secs(config.printer.command_timeout_seconds.max(5)),
            command.output(),
        )
        .await
        .map_err(|_| AppError::External("printer backend timed out".into()))??;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if output.status.success() {
            stdout
                .lines()
                .last()
                .map(str::to_string)
                .ok_or_else(|| AppError::External("printer backend returned no output".into()))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(AppError::External(format!(
                "printer backend failed: {}",
                if stderr.is_empty() { stdout } else { stderr }
            )))
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (config, args);
        Err(AppError::External(
            "real printing is only supported on Windows".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, BackendStatus};
    use crate::config::Config;

    #[test]
    fn paper_out_blocks_but_toner_only_warns() {
        let config = Config::default();
        let paper = classify(
            &config,
            BackendStatus {
                available: true,
                is_out_of_paper: true,
                ..empty()
            },
        );
        assert!(paper.blocked);
        let toner = classify(
            &config,
            BackendStatus {
                available: true,
                is_toner_low: true,
                ..empty()
            },
        );
        assert!(!toner.blocked);
        assert_eq!(toner.warnings.len(), 1);
    }

    fn empty() -> BackendStatus {
        BackendStatus {
            available: false,
            status: String::new(),
            is_offline: false,
            is_paused: false,
            is_in_error: false,
            is_not_available: false,
            need_user_intervention: false,
            is_out_of_paper: false,
            has_paper_problem: false,
            is_paper_jammed: false,
            is_toner_low: false,
            is_out_of_memory: false,
            is_output_bin_full: false,
            is_door_opened: false,
            is_manual_feed_required: false,
            is_server_unknown: false,
            jobs: vec![],
        }
    }
}
