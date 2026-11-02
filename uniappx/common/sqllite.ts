/**
 * 本地数据库模块（Android 走原生 SQLite，Web 走 SQLite 官方 WASM）
 *
 * 模块名保留英文 sqllite，对外接口全部使用中文，与 common/实时.ts 的风格保持一致。
 *
 * 职责：
 * 1. 打开本机数据库 im_renwu.db，并建好业务表（当前只有 会话表）；
 * 2. 把服务端的 消息会话 数据落到本地，供页面离线首显；
 * 3. 提供会话的增删改查、未读数字段维护。
 *
 * 平台差异（对外接口完全一致，只有初始化方式不同）：
 * 1. Android：底层是 uts 编译的原生 SQLite，全程同步，应用启动即可用；
 * 2. Web：底层是 SQLite 官方 WebAssembly（kvvfs/localStorage），
 *    WASM 必须异步加载，所以启动时要先 await 准备数据库() 一次；
 *    加载完成后，后面所有同步接口与 Android 一模一样；
 * 3. 其他平台（小程序等）没有本地数据库，调用会抛出明确错误，
 *    页面应当用 try/catch 兜住，不影响主流程。
 *
 * 调用约定：
 *   await 准备数据库()            // 启动时调一次，两端通用
 *   保存会话(...) / 查询会话列表(...)  // 之后全程同步，两端通用
 */

import type { 消息会话 } from './leixing'

// #ifdef APP-ANDROID || WEB 
import {
	打开数据库,
	关闭数据库,
	执行SQL,
	查询SQL,
	创建表,
	创建索引,
	表是否存在,
	数据库是否已打开,
	开启事务,
	提交事务,
	回滚事务,
	取得数据库文件路径
} from '@/uni_modules/cq-mqtt'
// #endif

// #ifdef WEB
// Web 端专属：WASM 的异步加载入口，Android 端没有这两个导出，必须分开导入
import { 准备SQLite, SQLite是否已就绪 } from '@/uni_modules/cq-mqtt'
// #endif

/* ---------------- 常量 ---------------- */

/** 数据库文件名，落在 /data/data/包名/databases/ 下 */
export const 数据库名 = 'im_renwu.db'

/** 数据库结构版本，表结构变更时 +1 */
export const 数据库版本 = 1

/** 会话表名 */
export const 会话表名 = '会话'

/* ---------------- 类型 ---------------- */

/**
 * 会话表的一行记录
 * 前 13 个字段与服务端 消息会话 对齐，后面 6 个是本地扩展字段
 */
export type 会话记录 = {
	/** 归属用户编号，用于多账号隔离 */
	归属用户编号 : string
	会话类型 : string
	会话编号 : string
	关联编号 : string
	会话名称 : string
	联系人昵称 : string | null
	会话说明 : string
	最近消息 : string | null
	最近消息时间 : string
	会话状态 : string | null
	成员数 : number | null
	邀请人编号 : string | null
	过期时间 : string | null
	MQTT主题 : string
	/** 未读消息数 */
	未读数 : number
	/** 是否置顶：0 否 1 是 */
	是否置顶 : number
	/** 是否免打扰：0 否 1 是 */
	是否免打扰 : number
	/** 未发送的草稿 */
	草稿 : string | null
	/** 本地最后更新时间（毫秒时间戳文本） */
	更新时间 : string
	/** 最后一次与服务端同步的时间（毫秒时间戳文本） */
	同步时间 : string
}

/** 初始化结果 */
export type 初始化结果 = {
	文件路径 : string
	版本 : number
	是否新建 : boolean
}

/* ---------------- 内部状态 ---------------- */

let 已初始化 = false

/* ---------------- 内部工具 ---------------- */

/** 没有原生 SQLite 时统一抛出这个错 */
function 抛出平台不支持() : void {
	throw new Error('当前平台不支持本地数据库，仅 Android 可用')
}

/** 取当前毫秒时间戳的文本形式 */
function 当前时间戳文本() : string {
	const 现在 = new Date()
	return '' + 现在.getTime()
}

/**
 * 把可能为空的文本统一成空串，避免往数据库里写 null
 * 入参必须写 any | null：UTS 的 any 编到 Kotlin 是非空 Any，
 * 声明成 any 时接收数据库读出的 any | null 会报「实际类型 Any?，预期类型 Any」
 */
function 文本(值 : any | null) : string {
	if (值 == null) {
		return ''
	}
	return '' + 值
}

/** 把可能为空的数字统一成 0，入参同样用 any | null */
function 数字(值 : any | null) : number {
	if (值 == null) {
		return 0
	}
	const 结果 = Number.parseFloat('' + 值)
	if (结果 != 结果) {
		return 0
	}
	return 结果
}

/** 取可空文本：数据库读出的是 any | null，不能直接赋给 string | null */
function 可空文本(值 : any | null) : string | null {
	if (值 == null) {
		return null
	}
	return '' + 值
}

/** 取可空数字：同理，不能把 any | null 直接赋给 number | null */
function 可空数字(值 : any | null) : number | null {
	if (值 == null) {
		return null
	}
	const 结果 = Number.parseFloat('' + 值)
	if (结果 != 结果) {
		return 0
	}
	return 结果
}

/** 把布尔转成 0 / 1，SQLite 没有布尔类型 */
function 是否(值 : boolean | null | undefined) : number {
	if (值 == true) {
		return 1
	}
	return 0
}

/* ---------------- 建表语句 ---------------- */

/** 会话表建表 SQL */
const 建会话表SQL = `
CREATE TABLE IF NOT EXISTS "会话" (
	"归属用户编号" TEXT NOT NULL,
	"会话类型" TEXT NOT NULL,
	"会话编号" TEXT NOT NULL,
	"关联编号" TEXT NOT NULL DEFAULT '',
	"会话名称" TEXT NOT NULL DEFAULT '',
	"联系人昵称" TEXT,
	"会话说明" TEXT NOT NULL DEFAULT '',
	"最近消息" TEXT,
	"最近消息时间" TEXT NOT NULL DEFAULT '',
	"会话状态" TEXT,
	"成员数" INTEGER,
	"邀请人编号" TEXT,
	"过期时间" TEXT,
	"MQTT主题" TEXT NOT NULL DEFAULT '',
	"未读数" INTEGER NOT NULL DEFAULT 0,
	"是否置顶" INTEGER NOT NULL DEFAULT 0,
	"是否免打扰" INTEGER NOT NULL DEFAULT 0,
	"草稿" TEXT,
	"更新时间" TEXT NOT NULL DEFAULT '',
	"同步时间" TEXT NOT NULL DEFAULT '',
	PRIMARY KEY ("归属用户编号", "会话编号")
)
`

/** 会话表索引：按最近消息时间倒序取会话列表 */
const 建会话时间索引SQL = 'CREATE INDEX IF NOT EXISTS "索引_会话_最近消息时间" ON "会话" ("归属用户编号", "最近消息时间" DESC)'

/** 会话表索引：置顶 + 时间，列表排序用 */
const 建会话置顶索引SQL = 'CREATE INDEX IF NOT EXISTS "索引_会话_置顶" ON "会话" ("归属用户编号", "是否置顶" DESC, "最近消息时间" DESC)'

/* ---------------- 平台兜底 ---------------- */

// #ifndef APP-ANDROID
// #ifndef WEB
function 打开数据库(参数 : any) : any | null { 抛出平台不支持(); return null }
function 关闭数据库(数据库名 : string) : void { 抛出平台不支持() }
function 执行SQL(参数 : any) : any | null { 抛出平台不支持(); return null }
function 查询SQL(参数 : any) : any | null { 抛出平台不支持(); return null }
function 创建表(参数 : any) : void { 抛出平台不支持() }
function 创建索引(参数 : any) : void { 抛出平台不支持() }
function 表是否存在(参数 : any) : boolean { 抛出平台不支持(); return false }
function 数据库是否已打开(数据库名 : string) : boolean { 抛出平台不支持(); return false }
function 开启事务(数据库名 : string) : void { 抛出平台不支持() }
function 提交事务(数据库名 : string) : void { 抛出平台不支持() }
function 回滚事务(数据库名 : string) : void { 抛出平台不支持() }
function 取得数据库文件路径(数据库名 : string) : string { 抛出平台不支持(); return '' }
// #endif
// #endif

/* ---------------- 初始化 ---------------- */

/** 建好全部业务表；打开数据库之后调用，Android 与 Web 共用 */
function 建业务表() : void {
	创建表({
		数据库名: 数据库名,
		表名: 会话表名,
		建表SQL: 建会话表SQL
	})
	执行SQL({ 数据库名: 数据库名, SQL: 建会话时间索引SQL })
	执行SQL({ 数据库名: 数据库名, SQL: 建会话置顶索引SQL })
	已初始化 = true
}

/**
 * 打开数据库并建好全部业务表，重复调用是安全的
 * 注意：Web 端必须先 await 准备数据库() 把 WASM 加载好，否则会抛错
 */
export function 初始化数据库() : 初始化结果 {
	// #ifndef APP-ANDROID
	// #ifndef WEB
	抛出平台不支持()
	return { 文件路径: '', 版本: 0, 是否新建: false }
	// #endif
	// #endif

	// #ifdef APP-ANDROID || WEB
	// #ifdef WEB
	if (SQLite是否已就绪() == false) {
		throw new Error('Web 端 SQLite WASM 尚未加载完成，请先 await 准备数据库()')
	}
	// #endif
	const 打开结果 = 打开数据库({
		数据库名: 数据库名,
		版本: 数据库版本
	})
	建业务表()
	const 结果 : 初始化结果 = {
		文件路径: 打开结果.文件路径,
		版本: 打开结果.版本,
		是否新建: 打开结果.是否新建
	}
	return 结果
	// #endif
}

/**
 * 异步初始化，两端通用，应用启动时调一次即可
 * Android：WASM 不存在，内部直接同步执行后 resolve
 * Web：先 await 加载 SQLite 官方 WASM，再建库建表
 */
export async function 准备数据库() : Promise<初始化结果> {
	// #ifdef WEB
	await 准备SQLite()
	// #endif
	return 初始化数据库()
}

/** 确保数据库已初始化；已打开时直接返回，页面不用每处都自己初始化 */
export function 确保已初始化() : void {
	if (已初始化 == true && 数据库是否已打开(数据库名) == true) {
		return
	}
	初始化数据库()
}

/** 关闭本地数据库，退出登录时可调用 */
export function 关闭本地数据库() : void {
	关闭数据库(数据库名)
	已初始化 = false
}

/** 取数据库文件的绝对路径，便于排查问题 */
export function 取得本地数据库路径() : string {
	确保已初始化()
	return 取得数据库文件路径(数据库名)
}

/** 会话表是否已经建好 */
export function 会话表是否已建() : boolean {
	确保已初始化()
	return 表是否存在({ 数据库名: 数据库名, 表名: 会话表名 })
}

/* ---------------- 写入 ---------------- */

/** 写入会话的 SQL，插入与整行覆盖共用这一条 */
const 插入会话SQL = 'INSERT OR REPLACE INTO "会话" (' +
	'"归属用户编号", "会话类型", "会话编号", "关联编号", "会话名称", "联系人昵称", ' +
	'"会话说明", "最近消息", "最近消息时间", "会话状态", "成员数", "邀请人编号", ' +
	'"过期时间", "MQTT主题", "未读数", "是否置顶", "是否免打扰", "草稿", ' +
	'"更新时间", "同步时间"' +
	') VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)'

/**
 * 把一条服务端会话转成插入参数数组
 * 元素类型必须是 any | null：UTS 的 any 编译到 Kotlin 是非空 Any，放 null 会编译报错
 */
function 会话转参数(归属用户编号 : string, 会话 : 消息会话) : Array<any | null> {
	const 时间戳 = 当前时间戳文本()
	return [
		归属用户编号,
		文本(会话.会话类型),
		文本(会话.会话编号),
		文本(会话.关联编号),
		文本(会话.会话名称),
		会话.联系人昵称 ?? null,
		文本(会话.会话说明),
		会话.最近消息 ?? null,
		文本(会话.最近消息时间),
		会话.会话状态 ?? null,
		会话.成员数 ?? null,
		会话.邀请人编号 ?? null,
		会话.过期时间 ?? null,
		文本(会话.MQTT主题),
		0,
		0,
		0,
		null,
		时间戳,
		时间戳
	]
}

/** 插入或更新一条会话（存在则整行覆盖），返回受影响行数 */
export function 保存会话(归属用户编号 : string, 会话 : 消息会话) : number {
	确保已初始化()
	const 结果 = 执行SQL({
		数据库名: 数据库名,
		SQL: 插入会话SQL,
		参数: 会话转参数(归属用户编号, 会话)
	})
	return 结果.受影响行数
}

/**
 * 在一个事务里批量写入会话，全成功才提交，返回写入条数
 * 说明：不走 在一个事务里执行()，是因为它的 语句集合 是 Array<SQL语句>，
 * UTS 的 type 编译到 Kotlin 是名义类型，any[] 传不进去（泛型不协变）
 */
export function 批量保存会话(归属用户编号 : string, 会话列表 : 消息会话[]) : number {
	确保已初始化()
	if (会话列表.length == 0) {
		return 0
	}
	开启事务(数据库名)
	try {
		for (let 序号 = 0; 序号 < 会话列表.length; 序号++) {
			执行SQL({
				数据库名: 数据库名,
				SQL: 插入会话SQL,
				参数: 会话转参数(归属用户编号, 会话列表[序号])
			})
		}
		提交事务(数据库名)
	} catch (异常) {
		回滚事务(数据库名)
		throw 异常
	}
	return 会话列表.length
}

/** 更新未读数 */
export function 更新未读数(归属用户编号 : string, 会话编号 : string, 未读数 : number) : number {
	确保已初始化()
	const 结果 = 执行SQL({
		数据库名: 数据库名,
		SQL: 'UPDATE "会话" SET "未读数"=?, "更新时间"=? WHERE "归属用户编号"=? AND "会话编号"=?',
		参数: [未读数, 当前时间戳文本(), 归属用户编号, 会话编号]
	})
	return 结果.受影响行数
}

/** 未读数累加，收到新消息时用；会话不存在则不做任何事 */
export function 累加未读数(归属用户编号 : string, 会话编号 : string, 增加数量 : number) : number {
	确保已初始化()
	const 结果 = 执行SQL({
		数据库名: 数据库名,
		SQL: 'UPDATE "会话" SET "未读数"="未读数"+?, "更新时间"=? WHERE "归属用户编号"=? AND "会话编号"=?',
		参数: [增加数量, 当前时间戳文本(), 归属用户编号, 会话编号]
	})
	return 结果.受影响行数
}

/** 未读数清零，进入会话页时用 */
export function 清零未读数(归属用户编号 : string, 会话编号 : string) : number {
	return 更新未读数(归属用户编号, 会话编号, 0)
}

/** 更新草稿 */
export function 更新草稿(归属用户编号 : string, 会话编号 : string, 草稿 : string | null) : number {
	确保已初始化()
	const 结果 = 执行SQL({
		数据库名: 数据库名,
		SQL: 'UPDATE "会话" SET "草稿"=?, "更新时间"=? WHERE "归属用户编号"=? AND "会话编号"=?',
		参数: [草稿, 当前时间戳文本(), 归属用户编号, 会话编号]
	})
	return 结果.受影响行数
}

/** 更新置顶 */
export function 更新置顶(归属用户编号 : string, 会话编号 : string, 是否置顶 : boolean) : number {
	确保已初始化()
	const 结果 = 执行SQL({
		数据库名: 数据库名,
		SQL: 'UPDATE "会话" SET "是否置顶"=?, "更新时间"=? WHERE "归属用户编号"=? AND "会话编号"=?',
		参数: [是否(是否置顶), 当前时间戳文本(), 归属用户编号, 会话编号]
	})
	return 结果.受影响行数
}

/** 更新免打扰 */
export function 更新免打扰(归属用户编号 : string, 会话编号 : string, 是否免打扰 : boolean) : number {
	确保已初始化()
	const 结果 = 执行SQL({
		数据库名: 数据库名,
		SQL: 'UPDATE "会话" SET "是否免打扰"=?, "更新时间"=? WHERE "归属用户编号"=? AND "会话编号"=?',
		参数: [是否(是否免打扰), 当前时间戳文本(), 归属用户编号, 会话编号]
	})
	return 结果.受影响行数
}

/** 删除一条会话 */
export function 删除会话(归属用户编号 : string, 会话编号 : string) : number {
	确保已初始化()
	const 结果 = 执行SQL({
		数据库名: 数据库名,
		SQL: 'DELETE FROM "会话" WHERE "归属用户编号"=? AND "会话编号"=?',
		参数: [归属用户编号, 会话编号]
	})
	return 结果.受影响行数
}

/** 清空某个账号的全部会话 */
export function 清空会话(归属用户编号 : string) : number {
	确保已初始化()
	const 结果 = 执行SQL({
		数据库名: 数据库名,
		SQL: 'DELETE FROM "会话" WHERE "归属用户编号"=?',
		参数: [归属用户编号]
	})
	return 结果.受影响行数
}

/* ---------------- 查询 ---------------- */

/** 把查询到的一行数据转成会话记录 */
function 行转会话(行 : Map<string, any | null>) : 会话记录 {
	return {
		归属用户编号: 文本(行.get('归属用户编号')),
		会话类型: 文本(行.get('会话类型')),
		会话编号: 文本(行.get('会话编号')),
		关联编号: 文本(行.get('关联编号')),
		会话名称: 文本(行.get('会话名称')),
		联系人昵称: 可空文本(行.get('联系人昵称')),
		会话说明: 文本(行.get('会话说明')),
		最近消息: 可空文本(行.get('最近消息')),
		最近消息时间: 文本(行.get('最近消息时间')),
		会话状态: 可空文本(行.get('会话状态')),
		成员数: 可空数字(行.get('成员数')),
		邀请人编号: 可空文本(行.get('邀请人编号')),
		过期时间: 可空文本(行.get('过期时间')),
		MQTT主题: 文本(行.get('MQTT主题')),
		未读数: 数字(行.get('未读数')),
		是否置顶: 数字(行.get('是否置顶')),
		是否免打扰: 数字(行.get('是否免打扰')),
		草稿: 可空文本(行.get('草稿')),
		更新时间: 文本(行.get('更新时间')),
		同步时间: 文本(行.get('同步时间'))
	}
}

/** 查询某个账号的全部会话，置顶优先、再按最近消息时间倒序 */
export function 查询会话列表(归属用户编号 : string) : 会话记录[] {
	确保已初始化()
	const 结果 = 查询SQL({
		数据库名: 数据库名,
		SQL: 'SELECT * FROM "会话" WHERE "归属用户编号"=? ORDER BY "是否置顶" DESC, "最近消息时间" DESC',
		参数: [归属用户编号]
	})
	const 列表 : 会话记录[] = []
	const 行集合 = 结果.行集合
	for (let 序号 = 0; 序号 < 行集合.length; 序号++) {
		列表.push(行转会话(行集合[序号]))
	}
	return 列表
}

/** 查询单条会话，不存在返回 null */
export function 查询单个会话(归属用户编号 : string, 会话编号 : string) : 会话记录 | null {
	确保已初始化()
	const 结果 = 查询SQL({
		数据库名: 数据库名,
		SQL: 'SELECT * FROM "会话" WHERE "归属用户编号"=? AND "会话编号"=? LIMIT 1',
		参数: [归属用户编号, 会话编号]
	})
	if (结果.行数 == 0) {
		return null
	}
	return 行转会话(结果.行集合[0])
}

/** 统计会话条数 */
export function 查询会话数量(归属用户编号 : string) : number {
	确保已初始化()
	const 结果 = 查询SQL({
		数据库名: 数据库名,
		SQL: 'SELECT COUNT(*) AS 总数 FROM "会话" WHERE "归属用户编号"=?',
		参数: [归属用户编号]
	})
	return 数字(结果.行集合[0].get('总数'))
}

/** 统计未读总数，用于底部tab角标 */
export function 查询未读总数(归属用户编号 : string) : number {
	确保已初始化()
	const 结果 = 查询SQL({
		数据库名: 数据库名,
		SQL: 'SELECT COALESCE(SUM("未读数"), 0) AS 未读总数 FROM "会话" WHERE "归属用户编号"=?',
		参数: [归属用户编号]
	})
	return 数字(结果.行集合[0].get('未读总数'))
}
