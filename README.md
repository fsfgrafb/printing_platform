# printing_platform

面向 SZUT ACM 实验室内网的自助打印平台。单个 Rust 进程提供网页、API、SQLite 数据存储和打印队列。

## 构建

需要 Rust stable：

```powershell
cargo build --release
```

Windows 产物：

```text
target\release\printing_platform.exe
```

Linux 产物：

```text
target/release/printing_platform
```

仓库中的 [config.toml](config.toml) 默认配置为：

- 监听 `0.0.0.0:80`
- 使用 `HP LaserJet Professional P1106`
- 启用真实打印

在仓库根目录运行：

```powershell
.\target\release\printing_platform.exe
```

浏览器访问打印主机 IP，例如 `http://192.168.1.100/`。

首次启动会创建默认管理员，学号、初始密码均为 `admin`；登录后须修改密码，最小长度由 `min_password_length` 配置。

## 配置

程序按以下顺序读取配置：

1. `PRINTING_PLATFORM_CONFIG` 指定的文件
2. 可执行文件同目录的 `config.toml`
3. 当前目录的 `config.toml`
4. 程序内置默认配置

主要配置项：

```toml
initial_admin_student_id = "admin"
min_password_length = 8
default_daily_limit = 10
session_days = 30
queue_poll_seconds = 5
cleanup_interval_hours = 6
file_retention_days = 365
temp_upload_retention_hours = 24

[server]
bind = "0.0.0.0:80"

[printer]
name = "HP LaserJet Professional P1106"
simulate = false
command_timeout_seconds = 60
job_discovery_seconds = 20
pdf_printer_path = ""

[converter]
office_program = ""
office_args = []
command_timeout_seconds = 180
```

`default_daily_limit` 用于初始化每日页数限额；管理员在系统设置中修改后，数据库中的设置会继续保留。

`printer.name` 必须与操作系统中的打印队列名称一致。`pdf_printer_path` 留空时，
Windows 会查找部署目录下的 `tools\SumatraPDF.exe`，再查找系统安装的
SumatraPDF 或 Adobe Reader。

PDF 可直接提交。Office、图片和 TXT 文件需要 LibreOffice。程序依次查找：

1. `converter.office_program`
2. 部署目录下的 `tools\LibreOffice\program\soffice.exe`
3. 部署目录下的 `tools\LibreOfficePortable\App\libreoffice\program\soffice.exe`
4. PATH 中的 `soffice.exe` 或 `libreoffice`

## 数据目录

默认数据位置：

- Windows：`%ProgramData%\PrintingPlatform`
- Linux：`$XDG_DATA_HOME/printing-platform` 或
  `~/.local/share/printing-platform`

可通过 `PRINTING_PLATFORM_DATA_DIR` 指定其他目录。程序会在其中保存 SQLite
数据库、上传文件、预览和运行时文件。

## Windows 部署

部署目录需要：

```text
printing_platform.exe
config.toml
tools\SumatraPDF.exe       # 未在系统安装 PDF 阅读器时需要
tools\LibreOffice\...      # 未在系统安装 LibreOffice 时需要
```

运行账号必须能访问目标打印机。按需在 Windows 防火墙中向受信任内网开放 TCP 80。

可通过任务计划程序设置开机启动。以管理员 PowerShell 执行：

```powershell
$InstallDir = 'C:\Program Files\PrintingPlatform'
$Credential = Get-Credential -Message '输入有权访问目标打印机的运行账号'
$Action = New-ScheduledTaskAction `
  -Execute "$InstallDir\printing_platform.exe" `
  -WorkingDirectory $InstallDir
$Trigger = New-ScheduledTaskTrigger -AtStartup
$Settings = New-ScheduledTaskSettingsSet `
  -RestartCount 5 `
  -RestartInterval (New-TimeSpan -Minutes 1) `
  -ExecutionTimeLimit (New-TimeSpan -Days 0)

Register-ScheduledTask `
  -TaskName 'PrintingPlatform' `
  -Action $Action `
  -Trigger $Trigger `
  -Settings $Settings `
  -User $Credential.UserName `
  -Password $Credential.GetNetworkCredential().Password `
  -RunLevel Highest

Start-ScheduledTask -TaskName 'PrintingPlatform'
```

查看日志时可先停止任务，再在 PowerShell 中直接运行程序：

```powershell
Stop-ScheduledTask -TaskName PrintingPlatform
& 'C:\Program Files\PrintingPlatform\printing_platform.exe'
```

## Linux 部署

安装并配置 CUPS 与 LibreOffice，确认服务账号能访问打印队列：

```bash
lpstat -p 'HP LaserJet Professional P1106'
```

安装程序和配置：

```bash
sudo useradd --system --home /var/lib/printing-platform \
  --shell /usr/sbin/nologin printing-platform
sudo usermod -a -G lp printing-platform
sudo install -Dm755 target/release/printing_platform \
  /opt/printing-platform/printing_platform
sudo install -d -o printing-platform -g printing-platform \
  /var/lib/printing-platform
sudo install -Dm640 -o root -g printing-platform config.toml \
  /etc/printing-platform/config.toml
```

创建 `/etc/systemd/system/printing-platform.service`：

```ini
[Unit]
Description=Printing Platform
After=network-online.target cups.service
Wants=network-online.target

[Service]
Type=simple
User=printing-platform
Group=printing-platform
SupplementaryGroups=lp
ExecStart=/opt/printing-platform/printing_platform
Environment=PRINTING_PLATFORM_CONFIG=/etc/printing-platform/config.toml
Environment=PRINTING_PLATFORM_DATA_DIR=/var/lib/printing-platform
Environment=HOME=/var/lib/printing-platform
Restart=on-failure
RestartSec=5
AmbientCapabilities=CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_BIND_SERVICE
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/printing-platform

[Install]
WantedBy=multi-user.target
```

启用服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now printing-platform
systemctl status printing-platform
journalctl -u printing-platform -f
```

不同发行版的 CUPS 组名可能不是 `lp`，请以本机配置为准。

## 健康检查

- `GET /api/health/live`：进程存活
- `GET /api/health/ready`：数据库可用
