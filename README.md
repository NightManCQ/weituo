# 任务发布与接单系统

包含 Rust + PostgreSQL + Apalis 后端和 Vue + MQTT 前端，当前交付为可编译运行的首期 MVP。

- 后端：[rust_houduan](./rust_houduan/)
- 前端：[vue_qianduan](./vue_qianduan/)
- 架构设计：[系统架构设计.md](./系统架构设计.md)
- Linux 后端构建与部署：[rust_houduan/部署Linux服务.md](./rust_houduan/部署Linux服务.md)
- Debian 服务模板：[部署/debian](./部署/debian/)

## 功能范围

- 账号注册/登录、Argon2id 密码摘要、2 小时 Bearer 访问令牌。
- 普通用户、运营员、管理员角色；运营员/管理员可以停用或恢复账号。
- 委托发布、浏览、接单、开始、提交完成、委托方验收并结算、取消、接单者释放；状态与积分账务使用同一数据库事务和行锁。
- 内部积分账户、可用/冻结余额、追加式交易流水；发布委托时托管积分，验收后发给接单者，取消或过期时退回发布者。
- 发布委托时，在同一事务内创建专属群聊并将发布者加入；接单时接单者加入现有群，释放时移除当前接单者，之后的新接单者不能查看上一轮聊天历史。验收完成时委托与群聊在同一事务中分别转为“已完成”和“已归档”。
- 好友私聊与普通群聊保持独立；群组、成员、邀请和消息分别存于 `"群_群组"`、`"群_成员"`、`"群_邀请"`、`"群_消息"`。委托群禁止邀请、移除和退出，成员只能由接单/释放委托流程变更。
- 消息页统一展示委托群聊、好友私聊和普通群聊；消息通过 HTTP 校验并持久化，随后服务端通过 MQTT 推送实时事件。浏览器只订阅和监听 MQTT，不发布聊天消息。
- 中文 PostgreSQL 业务表/字段。
- 事务发件箱和 Apalis PostgreSQL worker；定期处理过期任务，MQTT QoS 1 PUBACK 确认后才标记事件已投递。
- Vue 任务工作台和 MQTT.js 实时订阅。Broker JWT 按当前有效群成员关系与登录账号签发订阅 ACL。
- Vue 前端使用固定底部导航拆分为“任务、需求广场、发布需求、消息、群聊、联系人、我”；任务页默认打开，展示登录用户已接取的任务，个人页提供账号登录/注册和退出。

积分仅为站内记账单位，不可提现或用户间自由转让。新账号的初始积分为 0；运营员/管理员通过审计接口发放积分。历史委托的报酬为空且不可接单；新委托报酬要求为 1 至 1,000,000,000 的整数积分。系统不包含外部支付、管理员界面、文件附件、邮件/短信、刷新令牌、生产级速率限制与账号找回。部署前应按实际业务补充这些能力和相应测试。

## Rust 后端代码分层

`rust_houduan/src/` 按职责拆分启动配置、纯业务规则、HTTP 接口、共享数据模型和后台 worker；HTTP 接口按认证、任务、好友、群聊、实时连接和用户管理拆分为独立文件。任务状态机位于独立领域模块，不依赖 Axum 或数据库。源文件和核心业务规则优先采用中文命名；HTTP JSON 响应使用数据库中文字段名及中文业务属性名，包含认证响应、任务、群聊、实时凭据、管理结果和错误信息。请求体字段与查询参数保持现有英文命名以兼容客户端；JWT 标准声明和 MQTT 事件类型值按协议要求保留英文，MQTT 事件 JSON 属性使用中文。

例如登录响应使用 `访问令牌`、`令牌类型`、`有效秒数`、`用户`，其中用户字段为 `用户编号`、`登录名`、`昵称`；任务和聊天响应分别使用数据库字段名 `任务编号`、`任务状态`、`消息编号`、`消息正文` 等。Rust 响应结构体直接序列化中文字段名，不再通过 serde 将其改写成英文键。

- [启动.rs](./rust_houduan/src/启动.rs)：进程入口、数据库连接/迁移及 API/worker 模式分派（Rust 规定入口函数仍名为 `main`）。
- [配置.rs](./rust_houduan/src/配置.rs)：环境变量读取、校验和运行配置。
- [共享.rs](./rust_houduan/src/共享.rs)：跨层共享状态、请求/响应数据结构与错误映射。
- [接口.rs](./rust_houduan/src/接口.rs)：路由、HTTP 中间件、CORS 和监听地址。
- [接口/](./rust_houduan/src/接口)：认证、任务、好友、独立群聊、实时凭据、管理接口；群聊 HTTP 逻辑集中在 [群聊.rs](./rust_houduan/src/接口/群聊.rs)。
- `GET /api/v1/my/tasks`：需要登录，返回当前用户已接取的任务列表。
- `GET /api/v1/my/points`：需要登录，返回当前用户可用、冻结和总积分。
- `GET /api/v1/my/points/ledger?limit=50`：需要登录，返回最近积分流水及交易后余额。
- `POST /api/v1/admin/users/{id}/points`：运营员/管理员发放整数积分；需要 `Idempotency-Key`，请求体为 `{"amount": 100, "description": "..."}`，会记录操作审计。
- `POST /api/v1/tasks/{id}/complete`：接单者提交完成，委托进入“待验收”。
- `POST /api/v1/tasks/{id}/accept-completion`：发布者验收并发放已托管报酬，委托进入“已完成”。
- [领域/任务状态.rs](./rust_houduan/src/领域/任务状态.rs)：纯任务状态转换规则及领域单元测试。
- [工作者.rs](./rust_houduan/src/工作者.rs)：Apalis 调度、事务发件箱、过期任务、MQTT 聊天请求消费/校验和消息事件发布。

当前 MVP 的 HTTP 业务处理仍直接通过 SQLx 访问 PostgreSQL；业务规则已开始从 HTTP 层剥离。后续可随功能增长将 SQL 查询提取为 repository，并将跨表事务编排提取为 application service，避免为了分层而一次性引入不必要的抽象。

## Vue 前端代码分层

`vue_qianduan/src/` 将页面展示、工作台业务状态、HTTP 通信和领域数据类型分开；新增功能优先放入对应职责目录，避免继续把请求和业务规则堆入根页面。内部文件名、变量名、函数名和类型名优先使用中文；HTTP JSON 响应直接使用后端的中文字段名，发送到后端的请求体和查询参数仍按兼容契约使用英文键名。

- [App.vue](./vue_qianduan/src/App.vue)：应用外壳、页面切换和固定底部导航；具体页面各自维护在独立 Vue 单文件组件中。
- [页面/任务页.vue](./vue_qianduan/src/页面/任务页.vue)、[页面/需求广场页.vue](./vue_qianduan/src/页面/需求广场页.vue)、[页面/发布需求页.vue](./vue_qianduan/src/页面/发布需求页.vue)、[页面/消息页.vue](./vue_qianduan/src/页面/消息页.vue)、[页面/聊天页.vue](./vue_qianduan/src/页面/聊天页.vue)、[页面/个人中心页.vue](./vue_qianduan/src/页面/个人中心页.vue)：分别负责我的任务、需求广场、发布需求、会话列表、单独聊天和个人中心页面。
- [组合式/使用任务工作台.ts](./vue_qianduan/src/组合式/使用任务工作台.ts)：认证、任务操作、消息交互、MQTT 生命周期和页面业务状态。
- [接口/请求.ts](./vue_qianduan/src/接口/请求.ts)：统一 HTTP 请求、认证头、响应解析和错误处理。
- [领域/类型.ts](./vue_qianduan/src/领域/类型.ts)：用户、任务、消息等前端数据类型；协议字段与后端保持一致。

前端提交前运行 `npm run build`，该命令先执行 Vue/TypeScript 严格类型检查，再构建 Vite 生产资源。

## 本地开发启动与端口冲突

在 `rust_houduan` 目录运行 `cargo run` 会在同一进程中启动 API 和 worker，API 默认监听 `127.0.0.1:8080`，适合本地开发。也可显式指定 `cargo run -- all`。`cargo run -- api` 和 `cargo run -- worker` 仍可单独启动服务，适合部署为独立进程。同一台开发机不能同时启动两个监听相同地址和端口的 API；若收到“套接字地址只允许使用一次”，通常表示前一个 API 进程仍在运行，而不是数据库/MQTT 连接失败。

Windows PowerShell 中可定位占用端口的进程：

```powershell
$owner = (Get-NetTCPConnection -LocalPort 8080 -State Listen).OwningProcess
Get-Process -Id $owner
```

仅在确认该 PID 是自己启动的旧服务后再停止它，或在 `rust_houduan/.env` 将 `API_BIND` 改为其他可用地址。不要终止不认识的进程。日常开发可运行一次 `cargo run` 启动 API 和 worker，再运行前端 `npm run dev`。**实时聊天必须保持 API、worker、前端和 MQTT Broker 同时运行**：委托群聊与好友私聊均通过 HTTP 提交，API 返回成功表示消息已写入数据库；worker 负责扫描并重试未确认的发件箱事件。API 和 worker 都需要连接 MQTT Broker。部署时仍建议使用下方 systemd 模板分别运行 API 和 worker。

## 本机后端连接内网 PostgreSQL / EMQX

要求 Rust stable、Node.js 20+。`rust_houduan/.env.example` 已将 PostgreSQL、MQTT TCP 与 MQTT WebSocket 主机设为 `192.168.1.107`；同机浏览器默认通过 Vite 代理调用 `http://127.0.0.1:8080`。后端 `CORS_ORIGIN` 支持逗号分隔的多个浏览器来源，例如 `http://localhost:5173,http://127.0.0.1:5173`。若浏览器从局域网其他设备访问前端，需要将前端 `VITE_API_BASE` 指向运行后端电脑的局域网地址（例如 `http://192.168.1.103:8080/api/v1`），并将后端 `API_BIND` 设为该电脑的局域网 IP、`CORS_ORIGIN` 加入浏览器地址栏中实际使用的前端来源（例如 `http://192.168.1.103:5173`）。仅允许可信局域网访问该 API，并按需配置主机防火墙。项目 MQTT 使用独立端口 `1884`（TCP）和 `8085`（WebSocket），不修改服务器上其他项目的 EMQX 配置。端口可达不代表数据库/Broker 已认证或 ACL 可用；启动前须完成下面的项目专用 Broker 配置与验收。

1. 先在开发机创建配置文件：

   ```powershell
   cd rust_houduan
   Copy-Item .env.example .env
   ```

   编辑 `.env`：将 `DATABASE_URL` 的 `replace_me` 替换为 PostgreSQL 应用账号真实密码（URI 中对特殊字符做百分号编码）；把两个签名密钥换为不同的随机值，各至少 32 字节。`.env` 已被 Git 忽略，不要把它提交、粘贴到聊天或写入源文件。

2. 确认 PostgreSQL 已创建应用数据库和最小权限账号，并允许开发机到 5432 的访问。可使用专用 role/database 示例（须由 PostgreSQL 管理员执行，并把占位密码换成强随机值）：

   ```sql
   CREATE ROLE renwu_app LOGIN PASSWORD 'replace-with-a-strong-random-password';
   CREATE DATABASE "任务平台" OWNER renwu_app ENCODING 'UTF8';
   ```

   数据库服务器需在 `listen_addresses` 上监听服务器的内网接口，并在 `pg_hba.conf` 仅允许开发机内网 IP / 应用角色访问。不要允许 `0.0.0.0/0` 或对不可信网络开放。示例连接串已对中文数据库名做 URL 编码。
3. 在 `192.168.1.107` 上为本项目运行隔离的 EMQX 实例，不要改动现有其他项目的 Broker。使用 [docker-compose.emqx.yml](./部署/docker-compose.emqx.yml) 和 [emqx_acl.conf](./部署/emqx_acl.conf)，将两者复制到服务器目录 `/home/cq/renwu-emqx/`；在同目录创建权限为 `0600` 的 `.env`，填入与后端相同的 `MQTT_JWT_SECRET`，并设置独立强随机的 `EMQX_DASHBOARD_PASSWORD`、`EMQX_NODE_COOKIE`。运行 `docker compose --env-file .env -f docker-compose.emqx.yml up -d`。配置启用 HS256 JWT 和 JWT ACL，未匹配主题默认 deny；后端 worker 使用同一签名密钥生成仅允许发布的独立 JWT。开发端口为 TCP `1884`、WS `8085`，管理界面只绑定服务器本机。仅在可信内网开发时可使用未加密 WS；部署时配置 TLS/WSS 并将 `MQTT_PUBLIC_URL` 和前端 `VITE_MQTT_URL` 都改为 `wss://...`。
4. 在 `rust_houduan` 目录启动后端 API 和 worker（一个终端即可）：

   ```powershell
   cargo run
   ```

   默认同时运行 API 与 worker，启动时执行数据库迁移；健康检查为 `GET /health`。API 和 worker 需能连接 PostgreSQL，worker 还需连接项目专用 EMQX。需要单独运行时，可改用 `cargo run -- api` 或 `cargo run -- worker`。
5. 前端开发：

   ```powershell
   cd vue_qianduan
   npm ci --registry=https://registry.npmjs.org
   npm run dev
   ```

   浏览器打开 Vite 输出地址（默认 5173）。用邮箱以外的登录名注册账号，密码最少 10 字节。若 `.env` 使用自定义 WSS 地址，将相同 URL 放入前端 `.env` 的 `VITE_MQTT_URL`，然后重新运行 Vite。

### MQTT JWT 配置

本后端对每个已登录用户签发 60 秒 MQTT 专用 JWT，不复用 API Bearer token。按 [EMQX JWT 认证文档](https://docs.emqx.com/en/emqx/latest/guides/access-control/authn/jwt.html) 和 [JWT ACL 格式文档](https://docs.emqx.com/en/emqx/latest/guides/access-control/authn/acl.html) 配置：

1. 部署时只启用 WSS/TLS 客户端 listener；管理 Dashboard 限制为本机/管理网可达。开发时可用隔离 Broker 的内网 WS listener。
2. 创建从 MQTT `password` 读取 JWT 的 HMAC JWT authenticator，算法 HS256，密钥内容与 API 的 `MQTT_JWT_SECRET` 完全一致，关闭 Base64 secret 解码（除非密钥按 Base64 配置）。
3. 启用 JWT 的 `acl` claim 授权；未匹配的主题一律 deny。用户 JWT 的 `acl.pub` 为空，`acl.sub` 只包含当前有效成员所在且状态为“正常”的群聊主题 `q/{群聊id}` 和个人通知主题 `u/{用户编号}`。委托发布时即创建群聊，任务状态事件及群消息使用其 `q/{群聊id}`；验收后群聊归档，后续签发的凭据不再包含该主题。个人通知主题只使用稳定的用户 UUID，不使用登录名。实时令牌有效 60 秒，前端每 30 秒刷新凭据并重连；接单、验收后前端也会主动刷新，以尽快应用成员和归档变更。浏览器后台计时器可能被节流，严格即时撤权仍需 Broker 主动断开/撤销支持。前端不得发布 MQTT 消息、订阅其他用户主题或订阅任意通配符。
4. API 与 worker 共用后端发布凭据 `MQTT_USERNAME`，新事件仅向 `q/+` 和 `u/+` 发布。worker JWT 暂时保留 `renwu/+/events`，用于投递升级前已写入事务发件箱的旧任务事件，不再用于新事件或客户端订阅。群消息每条只写一条事务发件箱记录并发布到对应 `q/{群聊id}`；好友私聊和单用户通知发到每位接收者各自的 `u/{用户编号}`。数据库迁移 `202610280001_user_uuid_mqtt_topics.sql` 会将已存发件箱中的登录名个人主题转换为用户 UUID 主题；客户端不再订阅或回退到登录名主题。任务聊天 MQTT 请求消费者及其 `message_requests` / `message_receipts` 主题已移除；部署 Broker 时同步删除这些旧主题 ACL。
5. 浏览器看到 WebSocket `101 Switching Protocols` 只代表 WebSocket 握手成功；还需在 MQTT 网络帧/客户端日志中确认 MQTT `CONNACK` 成功、目标主题 `SUBACK` 获授权，并确认 worker 对应事件已发布。Broker 必须设置连接和消息大小限制。断线重连或首次打开消息页时仍通过 REST 加载历史会话；实时新消息由 MQTT 事件直接更新界面，不能将 MQTT 当作唯一存储。

首次接入前，应在本地或测试环境验证 MQTT JWT 的用户字段、ACL claim 和未匹配默认 deny 行为。Broker 具体设置界面/配置字段可能随 EMQX 版本改变，不能省略 ACL 验收。

## API 概览

所有 HTTP JSON 响应字段均使用中文；例如登录响应为 `{"访问令牌":"...","令牌类型":"Bearer","有效秒数":7200,"用户":{"用户编号":"...","登录名":"...","昵称":"..."}}`。任务、群聊、实时凭据、管理响应及错误响应也均使用对应中文数据库/业务字段名（错误信息键为 `消息`）。请求体字段和查询参数仍使用当前英文键名。

发布接口在创建委托的同一事务内创建关联群组并加入发布者；接单接口只加入当前接单者，不再创建群。`GET /api/v1/tasks/{id}/group-chat` 仅向有效委托群成员返回群组编号。释放委托会撤销原接单者的成员身份，下次接单时新成员从群内当前序号之后开始查看。验收完成时任务状态转为“已完成”、群组状态转为“已归档”，归档群禁止写入但保留授权成员的历史读取。迁移 `202610250003_commission_group_chat.sql` 将旧聊天历史复制到委托群；迁移 `202610260001_commission_group_lifecycle.sql` 为历史待接单委托补建关联群，并归档已完成委托群；先前迁移 `202610250004_remove_legacy_task_chat.sql` 已删除旧聊天表和旧任务聊天入口。好友私聊通过 `POST /api/v1/my/friends/{friend_id}/messages` 发送。事务发件箱之后经 MQTT 向 `u/{用户编号}` 发布私聊和单用户通知，向 `q/{群聊id}` 发布群消息及委托状态事件。消息页通过 `GET /api/v1/my/message-conversations` 一次获取活跃委托群、好友私聊和普通群聊会话，并按最近消息时间统一排序。

普通群聊由用户自主创建和管理；委托群使用相同的群组/成员/消息存储，但由委托状态机专门管理，禁止普通邀请、移除和退出。群消息通过 HTTP 持久化，同一事务写入一条 `group.message_created` outbox 事件，发布到 `q/{群聊id}`；群内序号由群组行锁定并递增分配。普通群首版人数上限 100，邀请好友确认后生效，新成员只能查看加入后的消息；HTTP 成功表示消息已保存，MQTT 负责实时通知。凭据只包含当前有效成员的群主题，成员变更在客户端每 30 秒刷新凭据并重连后生效。

| 方法 | 路径 | 登录要求 |
|---|---|---|
| POST | `/api/v1/auth/register` | 否 |
| POST | `/api/v1/auth/login` | 否 |
| GET | `/api/v1/tasks?status=待接单&limit=50` | 否 |
| POST | `/api/v1/tasks` | 是 |
| GET | `/api/v1/tasks/{id}` | 否 |
| GET | `/api/v1/weituo/{id}` | 否；按委托 ID 查询详情（拼音路径） |
| POST | `/api/v1/tasks/{id}/claim` | 是 |
| POST | `/api/v1/tasks/{id}/start` | 是 |
| POST | `/api/v1/tasks/{id}/complete` | 是 |
| POST | `/api/v1/tasks/{id}/accept-completion` | 是；发布者验收并发放积分 |
| POST | `/api/v1/tasks/{id}/cancel` | 是 |
| POST | `/api/v1/tasks/{id}/release` | 是 |
| GET | `/api/v1/my/tasks` | 是；查询当前用户已接取的任务 |
| GET | `/api/v1/my/message-conversations` | 是；统一查询委托、好友私聊和群聊会话 |
| GET | `/api/v1/tasks/{id}/group-chat` | 是；返回当前用户可访问的委托群组编号 |
| GET | `/api/v1/my/friends` | 是；查询好友列表 |
| GET | `/api/v1/my/friend-conversations` | 是；首次加载好友私聊会话 |
| POST | `/api/v1/my/friends/{friend_id}/messages` | 是；写入好友私聊消息 |
| POST | `/api/v1/groups` | 是；创建群组并成为群主 |
| GET | `/api/v1/my/groups` | 是；查询我的群组及最近消息摘要 |
| GET | `/api/v1/my/group-invitations` | 是；查询待处理群邀请 |
| GET | `/api/v1/groups/{group_id}` | 是；查询群详情及成员（仅群成员） |
| POST | `/api/v1/groups/{group_id}/invitations` | 是；群主/管理员邀请好友 |
| POST | `/api/v1/group-invitations/{invitation_id}/accept` | 是；接受邀请并加入群组 |
| POST | `/api/v1/group-invitations/{invitation_id}/reject` | 是；拒绝群邀请 |
| GET | `/api/v1/groups/{group_id}/messages?before_sequence=...&limit=50` | 是；查询群消息历史（仅有效成员） |
| POST | `/api/v1/groups/{group_id}/messages` | 是；幂等写入群消息并通知成员 |
| DELETE | `/api/v1/groups/{group_id}/members/{user_id}` | 是；群主/管理员移除成员 |
| DELETE | `/api/v1/groups/{group_id}/leave` | 是；退出群组（群主须先转让身份） |
| GET | `/api/v1/my/points` | 是；查询积分账户余额 |
| GET | `/api/v1/my/points/ledger` | 是；查询积分流水 |
| POST | `/api/v1/realtime/credentials` | 是 |
| POST | `/api/v1/admin/users/{id}/disable` | 运营员/管理员 |
| POST | `/api/v1/admin/users/{id}/enable` | 运营员/管理员 |
| POST | `/api/v1/admin/users/{id}/points` | 运营员/管理员；审计发放积分 |

注册用户默认只有“普通用户”角色。首次创建管理员后，应由数据库管理员在受控 SQL 会话中授予角色（不要将管理员密码或服务器凭证写进代码）：

```sql
INSERT INTO "用户角色" ("用户编号", "角色编号")
SELECT '替换为该用户的 UUID', "角色编号"
FROM "角色" WHERE "角色名称" = '管理员'
ON CONFLICT DO NOTHING;
```

所有写 API 均采用 JSON。认证请求头为 `Authorization: Bearer <access_token>`；创建任务和任务状态操作还必须提供稳定的 `Idempotency-Key`，重复请求不会重复产生任务状态事件。群聊消息使用客户端消息编号做幂等去重。

## Debian 部署概要

1. 先在目标 Debian 上检查 Debian 版本、PostgreSQL 版本、磁盘/内存和防火墙，再创建专用系统用户和 `renwu` PostgreSQL 角色/数据库。数据库只绑定 localhost 或私网。
2. 安装 Rust target 构建 release 二进制，编译 Vue 静态站点；安装 Nginx/Caddy 与 EMQX（按 EMQX 官方 Debian 安装文档选择与系统版本匹配的包）。
3. 依据上述 MQTT JWT/ACL 规则配置并先验收，再开放客户端 WSS；管理端口不可对公网开放。
4. 安装 [renwu-api.service](./部署/debian/renwu-api.service) 和 [renwu-worker.service](./部署/debian/renwu-worker.service)。将模板路径调整到实际安装位置，创建权限为 `0600` 的 `/etc/renwu/renwu.env`。
5. 配置 HTTPS/WSS 反向代理，前端只访问同源 API；公开端口仅保留 443（如需要证书签发再临时/受控开放 80）。
6. 首发前完成并发接单、聊天权限、broker 越权订阅、PostgreSQL 迁移/恢复演练与备份验证。

**安全提示：** 不要通过项目文件、命令行参数或聊天记录保存服务器口令；先更换任何弱口令，优先用 SSH 公钥和受限部署账号而非 root 密码登录。此代码库不包含服务器凭据，也不会自动更改现有服务器。
