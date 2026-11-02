# cq-mqtt

本插件目前提供 **Android 端 + Web 端的前端 SQLite 引擎**：
Android 走 uts 直接调用 `android.database.sqlite.*`（无三方依赖）；Web 走 SQLite 官方 WASM（kvvfs/localStorage 持久化）。
两端对外 API 逐字一致。

业务层不要直接拼 SQL，统一走 `common/sqllite.ts` 封装。

## 目录结构

```
utssdk/
├─ interface.uts          类型定义（同步参数 / 异步回调 / 结果）
├─ unierror.uts           错误码与文案
├─ app-android/
│  ├─ sqlite.uts          核心层：打开连接、执行 SQL、游标读取、事务
│  └─ index.uts           对外层：中文 API + 异步版本（io 线程干活，main 线程回调）
└─ web/
   ├─ sqlite.uts          核心层：官方 WASM 的 oo1 封装
   ├─ index.uts           对外层：与 Android 端逐字一致的 45 个 API + Web 专属初始化
   └─ sqlite-wasm/        官方产物（自包含）：index.mjs + sqlite3.wasm
```

## 对外 API（全部中文，同步 / 异步成对提供）

### 连接管理

| 接口 | 说明 |
| --- | --- |
| `打开数据库(参数)` | 打开或复用连接，返回 `{数据库名, 文件路径, 版本, 是否新建}` |
| `打开数据库异步(参数)` | 同上，io 线程执行 |
| `关闭数据库(数据库名)` / `关闭数据库异步(参数)` | 关闭连接 |
| `关闭全部数据库()` | 关闭全部连接，返回数量 |
| `删除数据库(数据库名)` / `删除数据库异步(参数)` | 删除数据库文件 |
| `数据库是否已打开(数据库名)` | 是否已打开 |
| `数据库文件是否存在(数据库名)` | 磁盘上是否有文件 |
| `取得数据库文件路径(数据库名)` | 绝对路径，便于排查 |
| `取得已打开数据库名列表()` | 当前打开的全部库名 |

### 执行与查询

| 接口 | 说明 |
| --- | --- |
| `执行SQL(参数)` | 写操作，返回 `{受影响行数, 最后插入编号}` |
| `批量执行SQL(参数)` | 多条写 SQL，不在事务中时自动包事务 |
| `查询SQL(参数)` | 返回 `{列名, 行集合, 行数}` |
| `查询单行(参数)` | 只取第一行，无结果返回 null |
| `查询单个值(参数)` | 只取第一行第一列 |
| `查询记录数(参数)` | COUNT 类查询，返回数字 |
| `执行SQL异步` / `批量执行SQL异步` / `查询SQL异步` | 对应的异步版本 |

### 事务

`开启事务(数据库名)`、`提交事务(数据库名)`、`回滚事务(数据库名)`、
`是否在事务中(数据库名)`、`在一个事务里执行(参数)`、`在一个事务里执行异步(参数)`

### 表结构与键值表

`创建表(参数)`、`表是否存在(参数)`、`删除表(参数)`、`清空表(参数)`、
`列是否存在(参数)`、`创建索引(参数)`、`删除索引(参数)`

`存储键值(参数)`、`读取键值(参数)`、`删除键值(参数)`、`清空键值(数据库名)`、`列出键(数据库名)`

### 版本与维护

`取得用户版本号(数据库名)`、`设置用户版本号(参数)`、`重组数据库(数据库名)`、
`重组数据库异步(参数)`、`写回预写日志(数据库名)`、`解析SQLite异常(异常)`

## 平台差异

| 能力 | Android（app-android） | Web（web） |
| --- | --- | --- |
| 底层 | `android.database.sqlite.*` | SQLite 官方 WASM（`@sqlite.org/sqlite-wasm`） |
| 存储 | `/data/data/包名/databases/库名` | kvvfs → `localStorage`（约 5MB） |
| 库数量 | 多个 | **只能一个**（kvvfs 限制，库名固定 `local`） |
| WAL | 支持 | 不支持（`写回预写日志()` 为空操作） |
| 初始化 | 打开即可用 | **必须先 `await 准备SQLite()`** |
| 导出 API | 45 个 | 同样 45 个（名字与签名完全一致） |

Web 端额外导出：`准备SQLite(地址?)`、`SQLite是否已就绪()`、`设置SQLite模块地址(地址)`、
`取得SQLite模块地址()`、`取得SQLite版本号()`、`取得Web存储占用()`。

### 为什么 Web 端不用 OPFS

按官方 [persistence.md](https://www.sqlite.org/wasm/doc/trunk/persistence.md)：
OPFS 只能在 **Worker 线程**里用，而 `"opfs"` / `"opfs-wl"` 还需要服务端下发
`Cross-Origin-Embedder-Policy: require-corp` + `Cross-Origin-Opener-Policy: same-origin`；
uni-app x 的 H5 跑在主线程，HBuilderX 的开发服务器也不带这两个头，
所以这里用官方的 **kvvfs（localStorage）** 方案：`new sqlite3.oo1.JsStorageDb('local')`。

### Web 端用法（WASM 必须异步加载）

```ts
// #ifdef WEB
import { 准备SQLite, 打开数据库, 执行SQL, 查询SQL } from '@/uni_modules/cq-mqtt'

await 准备SQLite()                        // 加载 sqlite3.wasm，幂等，可反复调用
打开数据库({ 数据库名: 'im_renwu.db' })     // 之后所有同步 API 与 Android 完全一致
// #endif
```

没有 `await 准备SQLite()` 就调同步 API，会抛 `[9010014]Web 端 SQLite 尚未就绪`。
所有 `*异步` 版本内部会自动 await，不需要调用方处理。

业务层（common/sqllite.ts）已经按下面的方式接好了，页面直接用即可：

```ts
await 准备数据库()            // 启动时一次：Android 同步执行；Web 先 await 准备SQLite() 再建库建表
查询会话列表(当前用户编号)      // 之后两端完全相同的同步调用
```

### WASM 资源位置（插件自带，四级兜底）

SQLite 官方产物随插件一起提供，位置 `utssdk/web/sqlite-wasm/`（含 `index.mjs` 与 `sqlite3.wasm`），
插件自身是完整的，不依赖项目任何目录。运行时按顺序尝试，谁先成功用谁：

| 顺序 | 来源 | 说明 |
| --- | --- | --- |
| 1 | 业务层注入的模块 | `注入SQLite模块(mod)`，完全绕开打包器 |
| 2 | 业务层指定的地址 | `设置SQLite模块地址(url)` |
| 3 | 插件自带 | `./sqlite-wasm/index.mjs`（首选，默认命中） |
| 4 | 项目静态目录副本 | `/static/sqlite/index.mjs`（可选兜底，验证通过后可删） |
| 5 | 官方 CDN | jsdelivr，内网不通时不适用 |

```ts
// 看实际尝试了哪些地址（排查资源加载问题用）
console.log(候选SQLite模块地址())

// 需要强制换地址时
设置SQLite模块地址('https://cdn.jsdelivr.net/npm/@sqlite.org/sqlite-wasm@3.54.1-build1/dist/index.mjs')
await 准备SQLite()

// 打包器把 import(变量) 处理坏时的逃生通道
import sqlite3Module from '@/uni_modules/cq-mqtt/utssdk/web/sqlite-wasm/index.mjs'
注入SQLite模块(sqlite3Module)
await 准备SQLite()
```

注意：`index.mjs` 与 `sqlite3.wasm` 必须放在同一目录，前者按 `import.meta.url` 定位后者。

## 错误处理

- 同步接口：失败直接 `throw Error`，错误信息形如 `[9010002]数据库 xxx 尚未打开，请先调用 打开数据库()`，业务层 `try/catch` 即可；
- 异步接口：失败走 `fail` 回调，入参为 `{errCode, errMsg, errSubject}`；
- 需要结构化错误时调用 `解析SQLite异常(异常)`。

错误码：9010001 参数缺失 / 9010002 未打开 / 9010003 无上下文 / 9010004 打开失败 /
9010005 执行失败 / 9010006 查询失败 / 9010007 关闭失败 / 9010008 删除失败 /
9010009 事务未开 / 9010010 事务中禁止 / 9010011 查询参数为 null / 9010012 键值失败 /
9010013 事务重复 / **9010014 Web 端未就绪（先 await 准备SQLite()）** / **9010015 Web 端不支持** /
9010099 未知错误。

## 注意事项

1. 查询参数不接受 `null`，遇到 `IS NULL` 条件请直接改写 SQL（Android `rawQuery` 的限制）；
2. `boolean` 落库为 `0` / `1`，读出来是数字；
3. 默认开启 WAL 与外键约束，可通过 `打开数据库` 的参数关闭；
4. 修改 UI 或响应式数据只能在 main 线程，异步接口的回调已经切回 main 线程。

### 开发文档

[UTS 语法](https://uniapp.dcloud.net.cn/tutorial/syntax-uts.html)
[UTS API插件](https://uniapp.dcloud.net.cn/plugin/uts-plugin.html)
[UTS uni-app兼容模式组件](https://uniapp.dcloud.net.cn/plugin/uts-component.html)
[UTS 标准模式组件](https://doc.dcloud.net.cn/uni-app-x/plugin/uts-vue-component.html)
[Hello UTS](https://gitcode.net/dcloud/hello-uts)
