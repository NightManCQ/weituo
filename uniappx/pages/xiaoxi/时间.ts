export function 解析日期(值 : string) : Date | null {
	if (!值) return null
	const 日期 = new Date(值)
	if (Number.isNaN(日期.getTime())) return null
	return 日期
}

export function 补零(值 : number) : string {
	return 值 < 10 ? `0${值}` : `${值}`
}

/**
 * 完整日期时间：2026年11月5日 18:00
 *
 * 不能用 toLocaleString('zh-CN', { dateStyle, timeStyle })：
 * 安卓运行时不支持这些选项，会退回默认格式（甚至原样返回 ISO 串），
 * web 端支持，于是两端显示不一致。这里全部用 getXxx 手工拼，两端一致。
 * 解析失败时原样返回，方便直接看出后端给了什么值。
 */
export function 格式化时间(值 : string) : string {
	const 日期 = 解析日期(值)
	if (日期 == null) return 值
	return `${日期.getFullYear()}年${日期.getMonth() + 1}月${日期.getDate()}日 ${补零(日期.getHours())}:${补零(日期.getMinutes())}`
}

/** 只到日期：2026-11-05 */
export function 格式化日期(值 : string) : string {
	const 日期 = 解析日期(值)
	if (日期 == null) return 值
	return `${日期.getFullYear()}-${补零(日期.getMonth() + 1)}-${补零(日期.getDate())}`
}

export function 格式化消息时间(值 : string) : string {
	const 日期 = 解析日期(值)
	if (日期 == null) return ''
	return `${String(日期.getHours()).padStart(2, '0')}:${String(日期.getMinutes()).padStart(2, '0')}`
}

export function 消息日期键(值 : string) : string {
	const 日期 = 解析日期(值)
	if (日期 == null) return 值.slice(0, 10)
	return `${日期.getFullYear()}-${日期.getMonth() + 1}-${日期.getDate()}`
}

export function 格式化日期标签(值 : string) : string {
	const 日期 = 解析日期(值)
	if (日期 == null) return 值.slice(0, 10)

	const 今天 = new Date()
	const 日期零点 = new Date(日期.getFullYear(), 日期.getMonth(), 日期.getDate()).getTime()
	const 今天零点 = new Date(今天.getFullYear(), 今天.getMonth(), 今天.getDate()).getTime()
	const 相差天数 = Math.floor((今天零点 - 日期零点) / 86400000)

	if (相差天数 == 0) return '今天'
	if (相差天数 == 1) return '昨天'
	return `${日期.getFullYear()}年${日期.getMonth() + 1}月${日期.getDate()}日`
}

export function 格式化会话时间(值 : string) : string {
	const 日期 = 解析日期(值)
	if (日期 == null) return ''

	const 今天 = new Date()
	const 日期零点 = new Date(日期.getFullYear(), 日期.getMonth(), 日期.getDate()).getTime()
	const 今天零点 = new Date(今天.getFullYear(), 今天.getMonth(), 今天.getDate()).getTime()
	const 相差天数 = Math.floor((今天零点 - 日期零点) / 86400000)

	if (相差天数 == 0) return 格式化消息时间(值)
	if (相差天数 == 1) return '昨天'
	if (日期.getFullYear() == 今天.getFullYear()) {
		return `${日期.getMonth() + 1}月${日期.getDate()}日`
	}
	return `${日期.getFullYear()}年${日期.getMonth() + 1}月${日期.getDate()}日`
}
