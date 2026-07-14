# ACM 实验室自助打印平台

Rust + axum + SQLite 后端，Vue 3 前端。项目用于实验室内网自助打印：用户上传文件、预览、提交队列；系统按每日页数限额控制，超额任务进入管理员审核。

代码内置配置默认使用模拟打印；仓库中的 `config.toml` 已按目标打印主机设置为真实模式。在开发机运行前请将 `printer.simulate` 临时改为 `true`。

## 环境准备

需要安装：

- Rust stable
- Node.js 与 npm
- Windows 生产环境需要 Print Spooler 服务正常运行
- 如需转换 Word/Excel/PPT，生产环境需要 Microsoft 365 或其他可调用的转换工具

如果当前 PowerShell 找不到 `node` 或 `npm`，但 Node.js 已安装在默认位置，可临时执行：

```powershell
$env:PATH='C:\Program Files\nodejs;' + $env:PATH
```

## 配置

主要配置文件是 `config.toml`：

```toml
database_url = "sqlite://data/printing_platform.db"
data_dir = "data"
initial_admin_student_id = "admin"
session_days = 365
queue_poll_seconds = 5
cleanup_interval_hours = 6
file_retention_days = 365
temp_upload_retention_hours = 24

[server]
bind = "127.0.0.1:8080"

[printer]
name = "HP LaserJet Professional P1106"
simulate = false
command_timeout_seconds = 60
backend_script = "scripts/printer-backend.ps1"
job_discovery_seconds = 20
pdf_printer_path = ""
```

开发时保持 `simulate = true`。真实提交到打印机时改为：

```toml
[printer]
name = "HP LaserJet Professional P1106"
simulate = false
```

如果要启用 Office COM 转 PDF：

```toml
[converter]
office_program = "powershell"
office_args = ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "scripts/convert-office.ps1", "-InputPath", "{input}", "-OutputPath", "{output}"]
command_timeout_seconds = 180
```

## 构建

### 复制到目标电脑的源文件

构建所需的最小文件集合如下：

```text
Cargo.toml
Cargo.lock
config.toml
src/
scripts/convert-office.ps1
scripts/printer-backend.ps1
frontend/index.html
frontend/package.json
frontend/package-lock.json
frontend/vite.config.js
frontend/src/
```

`README.md` 和 `docs/` 只用于部署参考，不参与构建。仓库只保留源代码和配置示例，不再提交 PDF 阅读器 exe；生产机请系统安装 SumatraPDF/Adobe Reader，或在本机自行放置未入库的 `tools/SumatraPDF.exe` 并通过 `printer.pdf_printer_path` 指定。

以下内容不要复制；它们会在构建、安装依赖或首次运行时重新产生：

```text
target/
frontend/node_modules/
frontend/dist/
data/
frontend/data/
tools/*.exe
.git/
```

如果目标电脑已有正式运行数据，应单独保留其 `data/`，不得用开发机数据覆盖。

### 后端检查与构建

```powershell
cargo fmt
cargo check
cargo build --release
```

构建产物位于：

```powershell
target\release\printing_platform.exe
```

### 前端依赖与构建

```powershell
cd frontend
npm install
npm run build
cd ..
```

前端生产产物会生成到 `frontend/dist`。本项目默认让前端服务占用 `80` 端口，后端监听本机 `8080`，前端把 `/api` 请求代理到后端。

## 启动

### 开发模式

启动后端：

```powershell
cargo run
```

后端默认监听：

```text
http://127.0.0.1:8080
```

另开一个终端启动前端开发服务：

```powershell
cd frontend
npm run dev
```

前端开发服务监听 `80` 端口，并代理 `/api` 到后端 `8080`。浏览器访问：

```text
http://127.0.0.1/
```

局域网内其他设备可访问打印主机 IP，例如：

```text
http://192.168.1.100/
```

如果 `80` 端口被 Steam++、IIS、Nginx 或其他服务占用，先关闭占用者再启动前端。

### 生产模式

先构建前端和后端：

```powershell
cd frontend
npm install
npm run build
cd ..
cargo build --release
```

启动服务：

```powershell
.\target\release\printing_platform.exe
```

另开终端启动前端静态服务：

```powershell
cd frontend
npm run preview
```

浏览器访问：

```text
http://10.18.47.101
```

也可以在本机访问：

```text
http://127.0.0.1
```

生产环境建议将 `printing_platform.exe` 注册为 Windows 服务，并把工作目录设置为项目根目录。前端可使用 `npm run preview` 临时运行在 `80` 端口；长期运行更建议使用 IIS、Nginx 或其他静态文件服务器占用 `80` 端口，并把 `/api` 反向代理到 `http://127.0.0.1:8080`。

## 首次使用

首次启动时，系统会检查是否已有管理员。如果没有，会根据 `config.toml` 的 `initial_admin_student_id` 自动创建管理员。

默认账号：

```text
学号：admin
密码：admin
```

如果你修改了 `initial_admin_student_id`，默认密码就是对应学号。首次登录后应立即在“个人设置”中修改密码。

## 普通用户如何使用

1. 使用学号和密码登录。
2. 首次登录后先修改密码。
3. 进入“提交打印”。
4. 拖拽文件到上传区松手，或点击上传区选择文件。
5. 等待系统生成 PDF 预览。
6. 选择打印范围：全部、奇数页、偶数页。
7. 点击“提交”。
8. 在“打印队列”查看任务状态和个人历史；可勾选“只看我的打印”。普通用户只能预览并下载自己的文件，其他人的文件名、预览与原始文件不可见。

限额规则：

- 默认每人每天 50 页。
- 超额任务不会直接打印，会进入“待审核”。
- 管理员同意后任务进入队列。
- 管理员拒绝后任务变为已取消。

取消规则：

- 普通用户只能取消自己的 `queued` 或 `pending_review` 任务。
- 已开始打印的任务不能由普通用户取消。

## 管理员如何使用

管理员登录后会看到额外菜单：

- 用户管理：导入用户、删除用户、重置密码、转让管理员，并查看各用户累计完成页数和任务数
- 审核中心：同意或拒绝超额任务
- 系统设置：修改限额和转让管理员

“打印队列”向所有登录用户显示全部用户近一年的记录，并支持按学号筛选和“只看我的打印”。普通用户不能查看他人文件名、最终 PDF 预览或原始文件下载；管理员可查看全部文件名、预览并下载原始文件，并额外拥有暂停/继续队列、取消他人任务和审批操作。

### 导入用户

进入“用户管理”，上传 Excel、CSV 或文本文件。

Excel 导入规则：

- 读取第一个工作表
- 读取第一列
- 每行一个学号
- 没有表头

新用户默认密码为学号，并要求首次登录修改密码。

### 管理队列

进入“打印队列”：

- “暂停”会阻止后续任务继续提交到打印机。
- 当前正在打印的任务不会被强制中断。
- “继续”会恢复队列调度。
- 管理员可取消未完成任务。
- 管理员可直接同意或拒绝待审核任务；审核中心保留同样的审批入口。

### 审核超额任务

进入“审核中心”：

- “同意”会把任务放回队列尾部。
- “拒绝”会把任务标记为已取消，并可填写原因。

### 导出统计

进入“用户管理”点击“导出统计”，或直接访问：

```text
/api/admin/stats.csv
```

需要管理员已登录。

## 数据与文件

运行时目录默认在 `data/`：

- `data/uploads/` 保存原始上传文件
- `data/previews/` 保存用于预览和打印的 PDF
- `data/tmp/` 保存导入等临时文件
- `data/printing_platform.db` 为 SQLite 数据库

自动清理策略：

- 24 小时前仍未提交的临时上传会被删除。
- 最终筛页并渲染好的打印 PDF 会与任务记录一起保留，登录用户可在打印队列中预览有权限的记录，也可下载同一条记录的原始文件。
- 365 天前已完成或已取消的任务记录及最终 PDF 等对应文件会一起删除。
- 清理间隔默认 6 小时。

这些时间可通过 `config.toml` 调整：

```toml
cleanup_interval_hours = 6
file_retention_days = 365
temp_upload_retention_hours = 24
```

## 文档转换与打印

PDF 文件会直接复制为预览文件。

非 PDF 文件会通过 `converter.office_program` 与 `converter.office_args` 真实转换并再次校验 PDF 页数。内置 PowerShell 脚本支持 Word、Excel、PowerPoint、JPG/JPEG、PNG、BMP 和 TXT；图片会先转换为灰度并按纸张方向等比缩放，再生成与实际黑白打印一致的 PDF 预览。浏览器会在上传前拦截不支持的扩展名，后端也会再次校验，不能绕过页面上传未知格式。未配置转换程序、转换超时或文件类型不支持时，上传会明确失败，不会生成占位内容。

真实打印：

- `printer.simulate = false` 时，后端通过 `scripts/printer-backend.ps1` 调用 PDF 命令行打印器，并捕获新建的 Windows 作业 ID。后端会优先使用 `printer.pdf_printer_path`，再查找本机自备的 `tools/SumatraPDF.exe`、系统安装的 SumatraPDF、Adobe Reader，最后才尝试系统 PDF `PrintTo` 关联。
- 调度器持续读取 `.NET System.Printing` 的具体状态和 PrintManagement 作业列表。缺纸、卡纸、脱机等状态会阻止提交后续任务，状态恢复后自动继续。
- 打印任务只有在已观察到的 Windows 作业从队列中结束后才标为 `done`；页面文案使用“Windows 打印作业已结束”，不承诺已物理打印成功。
- 墨粉不足仅向管理员告警，不阻塞队列；告警可确认，并在状态恢复后才允许下一次重新提示。
- 提交程序失败或无法确认作业 ID 时会暂停队列，避免自动重试造成重复打印。

### 打印队列状态

打印队列页向所有登录用户显示打印机名称、驱动原始状态和归一化状态：

- `Ready`：打印机可用且空闲，对应常见原始状态 `Normal`。
- `Running`：打印机正在初始化、处理或忙碌。
- `Printing`：驱动报告正在打印。
- `Paused`：管理员暂停了平台队列；不会继续提交新任务。
- `Offline`：打印机不可用或脱机。
- `Error`：缺纸、卡纸、舱门打开、驱动错误等需要处理的阻塞状态。

Windows 还可能返回 `Active`、`Processing`、`Busy`、`Initializing`、`Waiting`、`PaperOut`、`PaperJam`、`TonerLow` 等原始状态。页面保留原始值供排障，并将它们归并到上述状态；墨粉不足只告警，不阻塞队列。

打印机探测已经完成，原始 JSON 和一次性探测脚本不再随项目保留。最终依据见
[`docs/目标打印机能力与状态测试报告.md`](docs/目标打印机能力与状态测试报告.md)。

## 完成情况与验收边界

需求草案和早期后端结构草案已经合并进本 README、部署说明和现有实现，因此不再单独保留。目前业务接口、用户与管理员功能、额度审核、上传区拖放上传、文件转换、持久化队列、打印机状态阻塞、作业跟踪、提交页访问/累计打印页数展示、历史统计和清理任务均已实现。健康检查已合入路由入口，小型全局配置服务已内联到服务模块入口，避免为单一端点或薄封装保留额外源码文件。

仍需在目标电脑完成以下部署验收，不能仅用编译通过代替：

- 安装或配置 SumatraPDF/Adobe Reader，并用普通 PDF 验证命令行打印、Windows 作业捕获和实际出纸。
- 分别验证正常打印、缺纸暂停与补纸恢复、管理员取消作业。
- 使用目标服务账户运行 Office 文档转换，确认 Microsoft 365 COM 权限。
- 使用 NSSM、Windows `sc.exe` 或其他包装器完成正式服务注册和开机启动；项目本身不内置服务安装器。
- 墨粉状态仍属于驱动估算提示，不作为可靠阻塞条件。

## 常用接口

- `GET /api/health`：服务探活
- `POST /api/auth/login`：登录
- `POST /api/auth/logout`：退出登录
- `GET /api/auth/me`：获取当前用户
- `POST /api/print/upload`：上传文件并生成预览
- `POST /api/print/submit`：提交打印任务
- `GET /api/user/submit-stats`：提交页访问次数累加，并返回累计完成页数
- `GET /api/queue`：分页查看共享队列与历史，可传 `mine_only=true` 或 `student_id`
- `GET /api/print/tasks/:task_id/preview`：预览最终打印 PDF（管理员或任务本人可访问）
- `GET /api/print/tasks/:task_id/source`：下载原始上传文件（管理员或任务本人可访问）
- `GET /api/admin/users`：管理员查看用户及各用户累计完成页数、任务数
- `GET /api/admin/review`：管理员查看待审核任务
- `GET /api/admin/stats`：管理员查看统计
- `GET /api/admin/stats.csv`：管理员导出统计
- `POST /api/admin/printer/ack-toner`：管理员确认本轮墨粉提示

## 验证命令

后端：

```powershell
cargo fmt
cargo check
```

前端：

```powershell
cd frontend
npm install
npm run build
```

服务启动后可访问：

```text
http://127.0.0.1:8080/api/health
```

如果 `http://127.0.0.1/` 打不开，通常是前端服务没有启动或 `80` 端口被占用。开发模式启动顺序如下：

终端 1：

```powershell
cargo run
```

终端 2：

```powershell
cd frontend
npm run dev
```
