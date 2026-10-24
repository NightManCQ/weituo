# Rust 后端 Linux 构建与部署

本文说明如何将 `rust_houduan` 编译为 Linux 可执行文件，在 Linux 服务器上运行 API 和后台 worker，并通过 systemd 管理服务。示例以 Debian/Ubuntu、x86_64、systemd 为例；其他发行版请替换软件包管理命令。

## 1. 部署结构

建议 API 与 worker 分别作为 systemd 服务运行：

- API：监听 `127.0.0.1:8080`，由 Nginx/Caddy 通过 HTTPS 反向代理访问。
- Worker：处理过期委托、聊天消息和事务发件箱事件，需要访问 PostgreSQL 和项目的 MQTT Broker。
- PostgreSQL、MQTT Broker：建议使用已部署的服务或独立主机；不要为了部署本后端改动其他项目的服务配置。
- Vue 前端：单独构建并由 Nginx/Caddy 托管，或部署在其他静态站点服务上。

API 及 worker 启动时都会先连接 PostgreSQL 并运行内嵌的 SQLx 迁移。迁移文件在编译时嵌入二进制，因此部署新二进制时不需要单独复制 `migrations` 目录；首次部署/升级前仍应备份数据库并确认迁移影响。

## 2. 先准备 PostgreSQL 与 MQTT

部署前准备好：

1. PostgreSQL 数据库和专用应用账号。数据库账号需要能够连接数据库，并在启动迁移时拥有创建/修改业务表所需的权限。此项目当前使用同一个 `DATABASE_URL` 执行迁移和运行服务。
2. MQTT Broker 的 worker 连接地址、端口及 JWT 验证配置。Broker 的 JWT 签名密钥必须与后端 `MQTT_JWT_SECRET` 一致，并按项目要求启用 ACL；未匹配的 ACL 规则应拒绝访问。
3. 供浏览器连接的 MQTT WebSocket Secure 地址（`wss://...`）。它与 worker 的 `MQTT_HOST`/`MQTT_PORT` 是不同用途的配置。
4. 前端实际使用的站点 HTTPS 来源，用于 `CORS_ORIGIN`。

不要把 PostgreSQL 密码、JWT 密钥或高德安全密钥写入源代码、前端构建产物、命令行参数或版本库。数据库连接 URI 中的密码如含特殊字符，应先做百分号编码。

## 3. 在 Windows 打包源码

Windows 原生 `cargo build --release` 会生成 Windows `.exe`，不能直接在 Linux 上运行。应在 Linux 服务器、Linux CI，或目标 Linux 架构的 WSL2 环境中构建。下面采用“Windows 打包源码，Linux 服务器构建”的方式。

在 Windows PowerShell 执行以下命令。请确认生成的归档中没有本机 `.env` 或 `target` 目录：

```powershell
$ProjectRoot = 'C:\Users\cq\Documents\gong_zuo\jiaz_kz\renwu_xitong'
$Archive = Join-Path $env:TEMP 'renwu-linux-source.tar.gz'

tar.exe -czf $Archive `
  --exclude='rust_houduan/.env' `
  --exclude='rust_houduan/target' `
  -C $ProjectRoot `
  rust_houduan `
  '部署/debian/renwu-api.service' `
  '部署/debian/renwu-worker.service'

tar.exe -tzf $Archive | Select-String '(^|/)\.env$|/target/'
scp $Archive deploy-user@your-server:/tmp/renwu-linux-source.tar.gz
```

如果 `Select-String` 输出了实际 `.env` 或 `target` 内容，请不要上传；删除该归档并检查打包排除规则。`.env.example` 是不含真实密钥的模板，可以保留。将 `deploy-user` 和 `your-server` 替换为服务器 SSH 用户及主机名/IP。

## 4. 在 Linux 构建 release 二进制

通过 SSH 登录服务器。若使用 CI/WSL2 构建，请使用与服务器相同的 CPU 架构及兼容的 Linux ABI；不能把 Windows 的 `.exe` 上传运行。

```bash
sudo apt update
sudo apt install -y build-essential pkg-config ca-certificates tar gzip file

mkdir -p "$HOME/renwu-build"
tar -xzf /tmp/renwu-linux-source.tar.gz -C "$HOME/renwu-build"
cd "$HOME/renwu-build/rust_houduan"
```

安装 Rust stable 工具链（需要 Rust 1.85 或更新版本以支持 Rust 2024 edition）：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
rustup toolchain install stable
rustup default stable
rustc --version
cargo build --release
```

构建成功后，可执行文件位于：

```text
target/release/renwu-houduan
```

确认它是 Linux 可执行文件，而不是 Windows PE 文件：

```bash
file target/release/renwu-houduan
```

若在 WSL2/CI 构建，再将这个 Linux 二进制传到服务器，例如：

```powershell
scp 'C:\path\to\linux-build\renwu-houduan' deploy-user@your-server:/tmp/renwu-houduan
```

这里的 `C:\path\to\linux-build\renwu-houduan` 是 WSL/CI 导出的 Linux 构建产物路径；不要指向 Windows 原生 `target\release\renwu-houduan.exe`。

## 5. 安装服务账号与文件

在服务器使用有 sudo 权限的账号执行：

```bash
sudo useradd --system --home-dir /var/lib/renwu --create-home \
  --shell /usr/sbin/nologin renwu
```

如果 `renwu` 用户已经存在，跳过 `useradd`。创建目录并安装二进制：

```bash
sudo install -d -o root -g root -m 0755 /opt/renwu/bin
sudo install -d -o renwu -g renwu -m 0750 /var/lib/renwu
sudo install -o root -g root -m 0755 \
  "$HOME/renwu-build/rust_houduan/target/release/renwu-houduan" \
  /opt/renwu/bin/renwu-houduan
```

如果二进制从其他 Linux 构建机上传，最后一条命令改为：

```bash
sudo install -o root -g root -m 0755 \
  /tmp/renwu-houduan /opt/renwu/bin/renwu-houduan
```

## 6. 配置运行环境

创建仅 root 可读的环境文件：

```bash
sudo install -d -o root -g root -m 0750 /etc/renwu
sudo touch /etc/renwu/renwu.env
sudo chown root:root /etc/renwu/renwu.env
sudo chmod 0600 /etc/renwu/renwu.env
sudoedit /etc/renwu/renwu.env
```

按实际环境填写以下配置。所有示例值都必须替换；不要复用文档或开发环境中的秘密：

```ini
DATABASE_URL=postgres://renwu_app:URL_ENCODED_PASSWORD@127.0.0.1:5432/ren_wu_pt
JWT_SECRET=REPLACE_WITH_AT_LEAST_32_RANDOM_BYTES
API_BIND=127.0.0.1:8080
CORS_ORIGIN=https://your-frontend.example.com
RUST_LOG=renwu_houduan=info,tower_http=info

MQTT_HOST=127.0.0.1
MQTT_PORT=1884
MQTT_USERNAME=renwu_worker
MQTT_CLIENT_ID=renwu-backend-worker
MQTT_TLS=false
MQTT_PUBLIC_URL=wss://mqtt.example.com/mqtt
MQTT_JWT_SECRET=REPLACE_WITH_A_DIFFERENT_RANDOM_SECRET

AMAP_SECURITY_CODE=REPLACE_WITH_AMAP_SERVER_SECURITY_CODE
```

说明：

- `JWT_SECRET` 和 `MQTT_JWT_SECRET` 必须不同，长度均至少 32 字节。可在服务器分别生成随机值：`openssl rand -hex 32`。在受保护的终端/密码管理器中处理，不要把输出粘贴到工单或日志。
- `DATABASE_URL` 应指向实际数据库；若数据库在其他主机，将 `127.0.0.1` 替换为数据库主机地址。
- `API_BIND` 绑定本机回环地址，避免绕过 HTTPS 反向代理直接将 API 暴露到公网。仅当有明确的防火墙和访问控制需求时才绑定其他地址。
- `CORS_ORIGIN` 必须是浏览器前端的准确 origin（协议、主机、端口），末尾不要加 API 路径。
- `MQTT_HOST` 和 `MQTT_PORT` 是 worker 到 Broker 的服务端连接地址；`MQTT_PUBLIC_URL` 是后端返回给浏览器的 WebSocket 地址，生产应使用 `wss://`。
- `MQTT_TLS=true` 表示 worker 到 MQTT Broker 的连接也启用 TLS；应与 Broker TCP listener 的配置相符。
- `AMAP_SECURITY_CODE` 是可选的后端高德 JS API 安全密钥。前端 JS API Key 应配置在 `vue_qianduan/.env` 的 `VITE_AMAP_KEY`，不要放进本文件以外的前端源码。
- systemd 的 `EnvironmentFile` 不是 shell 脚本，不要在其中写 `export`、变量展开或命令替换。

## 7. 安装并启动 systemd 服务

源码归档包含项目现有的服务模板。安装模板：

```bash
sudo install -m 0644 \
  "$HOME/renwu-build/部署/debian/renwu-api.service" \
  /etc/systemd/system/renwu-api.service
sudo install -m 0644 \
  "$HOME/renwu-build/部署/debian/renwu-worker.service" \
  /etc/systemd/system/renwu-worker.service
sudo systemctl daemon-reload
```

启动 API 与 worker 并设置开机启动：

```bash
sudo systemctl enable --now renwu-api.service
sudo systemctl status renwu-api.service --no-pager
curl --fail http://127.0.0.1:8080/health

sudo systemctl enable --now renwu-worker.service
sudo systemctl status renwu-worker.service --no-pager
```

健康检查成功时返回 `{"状态":"ok"}`。服务日志可通过以下命令查看：

```bash
sudo journalctl -u renwu-api.service -n 100 --no-pager
sudo journalctl -u renwu-worker.service -n 100 --no-pager
sudo journalctl -u renwu-api.service -f
```

如果启动失败，先看日志。常见原因包括缺少 `DATABASE_URL`/JWT 配置、数据库网络或权限错误、迁移失败、MQTT JWT/ACL 配置不匹配，以及 8080 端口被占用。不要通过关闭防火墙或开放数据库到公网来绕过连接问题。

## 8. HTTPS 反向代理与前端连接

将 Nginx/Caddy 配置为提供 HTTPS，并将 API 请求代理到本机：

- `/api/` → `http://127.0.0.1:8080`
- `/_AMapService/` → `http://127.0.0.1:8080`（前端使用后端高德代理时）
- `/health` 可仅限本机/监控网络访问，避免向公网暴露不必要的诊断端点。

保留原始请求路径和 `Host`、`X-Forwarded-For`、`X-Forwarded-Proto` 等代理头。公网只开放所需的 HTTPS/WSS 端口；API 8080、PostgreSQL 和 MQTT TCP 管理端口不要直接暴露公网。

前端构建时配置：

- `VITE_API_BASE=/api/v1`（同源反向代理时）或实际 HTTPS API URL。
- `VITE_MQTT_URL=wss://mqtt.example.com/mqtt`，须与 `MQTT_PUBLIC_URL` 对应。
- `VITE_AMAP_KEY` 设置为高德开放平台配置的 Web JS API Key，并限制允许的域名。

`VITE_*` 环境变量会写入前端构建资源，不能放任何私钥或后端安全密钥。

## 9. 更新版本

更新前先备份 PostgreSQL。尤其是数据库迁移可能改变表结构时，应先在备份/测试库验证。建议停掉两个服务，替换二进制后先启动 API（执行迁移并检查健康），再启动 worker：

```bash
sudo systemctl stop renwu-worker.service renwu-api.service

# 将新 release 二进制安装到临时文件，再原子替换，避免写入中断
sudo install -o root -g root -m 0755 \
  "$HOME/renwu-build/rust_houduan/target/release/renwu-houduan" \
  /opt/renwu/bin/renwu-houduan.new
sudo mv /opt/renwu/bin/renwu-houduan.new /opt/renwu/bin/renwu-houduan

sudo systemctl start renwu-api.service
sudo systemctl status renwu-api.service --no-pager
curl --fail http://127.0.0.1:8080/health
sudo systemctl start renwu-worker.service
sudo systemctl status renwu-worker.service --no-pager
```

不要在迁移失败后直接删除迁移记录或手工回退数据库；先保存日志和数据库备份，再按失败迁移的具体情况制定恢复方案。

## 10. 常用运维命令

```bash
sudo systemctl restart renwu-api.service
sudo systemctl restart renwu-worker.service
sudo systemctl stop renwu-api.service renwu-worker.service
sudo systemctl is-enabled renwu-api.service renwu-worker.service
sudo systemctl is-active renwu-api.service renwu-worker.service
```

变更 `/etc/renwu/renwu.env` 后，重启对应服务使配置生效。密钥轮换时需同步更新 Broker 的 MQTT JWT 验证配置，并安排 API、worker 与 Broker 的协调重启。定期备份数据库、限制备份访问，并在恢复演练中验证委托、积分账户、交易/流水和迁移状态的一致性。
