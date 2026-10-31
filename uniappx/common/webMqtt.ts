import type { MQTT事件, MQTT事件数据, MQTT回调, MQTT连接参数 } from './实时.类型'

let 连接对象 : ReturnType<typeof uni.connectSocket> | null = null
let 已连接 = false
let 消息编号 = 1
let 心跳定时器 : number | null = null
let 连接超时定时器 : number | null = null
let 连接完成 : ((原因 ?: Error) => void) | null = null
const 等待订阅确认 = new Map<number, { resolve: () => void; reject: (原因: Error) => void }>()
const 等待取消订阅确认 = new Map<number, { resolve: () => void; reject: (原因: Error) => void }>()
const 等待发布确认 = new Map<number, { resolve: () => void; reject: (原因: Error) => void }>()

const 事件监听器 = new Map<MQTT事件, Set<MQTT回调>>()

function 触发事件(事件 : MQTT事件, 数据 : MQTT事件数据 = {}) {
	事件监听器.get(事件)?.forEach((回调) => {
		try {
			回调(数据)
		} catch (原因) {
			console.error('MQTT 事件监听处理失败', 原因)
		}
	})
}

export function 注册事件监听(事件 : MQTT事件, 回调 : MQTT回调) {
	if (!事件监听器.has(事件)) 事件监听器.set(事件, new Set<MQTT回调>())
	事件监听器.get(事件)?.add(回调)
}

export function 移除事件监听(事件 : MQTT事件, 回调 ?: MQTT回调) {
	if (回调 == null) 事件监听器.delete(事件)
	else 事件监听器.get(事件)?.delete(回调)
}

function 编码UTF8(文本 : string) : number[] {
	const 字节 : number[] = []
	for (let 索引 = 0; 索引 < 文本.length; 索引++) {
		let 码点 = 文本.charCodeAt(索引)
		if (码点 >= 0xD800 && 码点 <= 0xDBFF && 索引 + 1 < 文本.length) {
			const 低位 = 文本.charCodeAt(索引 + 1)
			if (低位 >= 0xDC00 && 低位 <= 0xDFFF) {
				码点 = ((码点 - 0xD800) << 10) + (低位 - 0xDC00) + 0x10000
				索引++
			}
		}
		if (码点 < 0x80) 字节.push(码点)
		else if (码点 < 0x800) {
			字节.push(0xC0 | (码点 >> 6), 0x80 | (码点 & 0x3F))
		} else if (码点 < 0x10000) {
			字节.push(0xE0 | (码点 >> 12), 0x80 | ((码点 >> 6) & 0x3F), 0x80 | (码点 & 0x3F))
		} else {
			字节.push(
				0xF0 | (码点 >> 18),
				0x80 | ((码点 >> 12) & 0x3F),
				0x80 | ((码点 >> 6) & 0x3F),
				0x80 | (码点 & 0x3F),
			)
		}
	}
	return 字节
}

function 解码UTF8(字节 : Uint8Array) : string {
	let 编码 = ''
	for (let 索引 = 0; 索引 < 字节.length; 索引++) {
		编码 += `%${字节[索引].toString(16).padStart(2, '0')}`
	}
	try {
		return decodeURIComponent(编码)
	} catch (原因) {
		console.error('MQTT 消息 UTF-8 解码失败', 原因)
		return ''
	}
}

function MQTT字符串字段(文本 : string) : number[] {
	const 字节 = 编码UTF8(文本)
	return [(字节.length >> 8) & 0xFF, 字节.length & 0xFF, ...字节]
}

function 编码剩余长度(长度 : number) : number[] {
	const 字节 : number[] = []
	do {
		let 当前 = 长度 % 128
		长度 = Math.floor(长度 / 128)
		if (长度 > 0) 当前 |= 0x80
		字节.push(当前)
	} while (长度 > 0)
	return 字节
}

function MQTT数据包(头 : number, 内容 : number[]) : ArrayBuffer {
	return new Uint8Array([头, ...编码剩余长度(内容.length), ...内容]).buffer
}

function 获取消息编号() : number {
	const 编号 = 消息编号
	消息编号 = 消息编号 >= 65535 ? 1 : 消息编号 + 1
	return 编号
}

function 发送(数据 : ArrayBuffer) {
	连接对象?.send({ data: 数据 })
}

export function 订阅(主题 : string | string[], qos : 0 | 1 | 2 = 0) : Promise<void> {
	if (!已连接) return Promise.reject(new Error('MQTT 尚未连接'))
	const 主题列表 = Array.isArray(主题) ? 主题 : [主题]
	if (主题列表.length == 0) return Promise.resolve()

	const 编号 = 获取消息编号()
	const 内容 = [(编号 >> 8) & 0xFF, 编号 & 0xFF]
	主题列表.forEach((项) => 内容.push(...MQTT字符串字段(项), qos))
	return new Promise<void>((resolve, reject) => {
		等待订阅确认.set(编号, { resolve, reject })
		发送(MQTT数据包(0x82, 内容))
	})
}

export function 取消订阅(主题 : string | string[]) : Promise<void> {
	if (!已连接) return Promise.reject(new Error('MQTT 尚未连接'))
	const 主题列表 = Array.isArray(主题) ? 主题 : [主题]
	if (主题列表.length == 0) return Promise.resolve()

	const 编号 = 获取消息编号()
	const 内容 = [(编号 >> 8) & 0xFF, 编号 & 0xFF]
	主题列表.forEach((项) => 内容.push(...MQTT字符串字段(项)))
	return new Promise<void>((resolve, reject) => {
		等待取消订阅确认.set(编号, { resolve, reject })
		发送(MQTT数据包(0xA2, 内容))
	})
}

export function 发布(主题 : string, payload : string | ArrayBuffer, 选项 ?: { qos?: 0 | 1 | 2 }) : Promise<void> {
	if (!已连接) return Promise.reject(new Error('MQTT 尚未连接'))
	const 服务质量 = 选项?.qos ?? 0
	const 内容字节 = typeof payload == 'string' ? 编码UTF8(payload) : Array.from(new Uint8Array(payload))
	const 主题字段 = MQTT字符串字段(主题)
	const 编号 = 服务质量 > 0 ? 获取消息编号() : null
	const 消息内容 = 编号 == null
		? [...主题字段, ...内容字节]
		: [...主题字段, (编号 >> 8) & 0xFF, 编号 & 0xFF, ...内容字节]
	if (编号 == null) {
		发送(MQTT数据包(0x30, 消息内容))
		return Promise.resolve()
	}
	return new Promise<void>((resolve, reject) => {
		等待发布确认.set(编号, { resolve, reject })
		发送(MQTT数据包(0x30 | (服务质量 << 1), 消息内容))
	})
}

function 处理数据包(数据 : ArrayBuffer) {
	const 字节 = new Uint8Array(数据)
	let 游标 = 0
	while (游标 + 2 <= 字节.length) {
		const 头 = 字节[游标++]
		let 剩余长度 = 0
		let 倍数 = 1
		let 长度完成 = false
		for (let 次数 = 0; 次数 < 4 && 游标 < 字节.length; 次数++) {
			const 当前 = 字节[游标++]
			剩余长度 += (当前 & 0x7F) * 倍数
			倍数 *= 128
			if ((当前 & 0x80) == 0) {
				长度完成 = true
				break
			}
		}
		if (!长度完成 || 游标 + 剩余长度 > 字节.length) return

		const 内容开始位置 = 游标
		const 内容结束位置 = 游标 + 剩余长度
		const 类型 = 头 >> 4
		if (类型 == 2) {
			const 返回码 = 剩余长度 >= 2 ? 字节[内容开始位置 + 1] : -1
			if (返回码 != 0) {
				const 原因 = new Error(`MQTT broker 拒绝连接（返回码 ${返回码}）`)
				连接完成?.(原因)
				连接完成 = null
				触发事件('error', { error: 原因.message })
				连接对象?.close({})
				return
			}
			已连接 = true
			if (心跳定时器 != null) clearInterval(心跳定时器)
			心跳定时器 = setInterval(() => 发送(new Uint8Array([0xC0, 0]).buffer), 45000)
			连接完成?.()
			连接完成 = null
			触发事件('connect')
		} else if (类型 == 9 || 类型 == 11 || 类型 == 4 || 类型 == 5 || 类型 == 6 || 类型 == 7) {
			if (剩余长度 < 2) return
			const 编号 = (字节[内容开始位置] << 8) | 字节[内容开始位置 + 1]
			if (类型 == 9) {
				const 等待项 = 等待订阅确认.get(编号)
				等待订阅确认.delete(编号)
				if (剩余长度 >= 3 && 字节[内容结束位置 - 1] == 0x80) 等待项?.reject(new Error('MQTT broker 拒绝订阅'))
				else 等待项?.resolve()
			} else if (类型 == 11) {
				const 等待项 = 等待取消订阅确认.get(编号)
				等待取消订阅确认.delete(编号)
				等待项?.resolve()
			} else if (类型 == 4) {
				const 等待项 = 等待发布确认.get(编号)
				等待发布确认.delete(编号)
				等待项?.resolve()
			} else if (类型 == 5) {
				发送(MQTT数据包(0x62, [(编号 >> 8) & 0xFF, 编号 & 0xFF]))
			} else if (类型 == 6) {
				发送(MQTT数据包(0x70, [(编号 >> 8) & 0xFF, 编号 & 0xFF]))
			} else if (类型 == 7) {
				const 等待项 = 等待发布确认.get(编号)
				等待发布确认.delete(编号)
				等待项?.resolve()
			}
		} else if (类型 == 3) {
			if (剩余长度 < 2) return
			const 主题长度 = (字节[内容开始位置] << 8) | 字节[内容开始位置 + 1]
			let 内容游标 = 内容开始位置 + 2
			if (内容游标 + 主题长度 > 内容结束位置) return
			const 主题 = 解码UTF8(字节.slice(内容游标, 内容游标 + 主题长度))
			内容游标 += 主题长度
			const 服务质量 = (头 >> 1) & 0x03
			let 本次消息编号 = 0
			if (服务质量 > 0) {
				if (内容游标 + 2 > 内容结束位置) return
				本次消息编号 = (字节[内容游标] << 8) | 字节[内容游标 + 1]
				内容游标 += 2
			}
			if (服务质量 == 1) {
				发送(MQTT数据包(0x40, [(本次消息编号 >> 8) & 0xFF, 本次消息编号 & 0xFF]))
			} else if (服务质量 == 2) {
				发送(MQTT数据包(0x50, [(本次消息编号 >> 8) & 0xFF, 本次消息编号 & 0xFF]))
			}
			触发事件('message', {
				topic: 主题,
				payload: 解码UTF8(字节.slice(内容游标, 内容结束位置)),
			})
		}
		游标 = 内容结束位置
	}
}

export function 连接(参数 : MQTT连接参数) : Promise<void> {
	if (连接对象 != null) return Promise.reject(new Error('Web MQTT 连接正在使用中'))

	return new Promise<void>((resolve, reject) => {
		let 新连接 : ReturnType<typeof uni.connectSocket> | null = null
		const 连接失败 = (原因 : Error) => {
			if (连接完成 == null) return
			连接完成 = null
			if (连接超时定时器 != null) {
				clearTimeout(连接超时定时器)
				连接超时定时器 = null
			}
			reject(原因)
		}
		连接完成 = (原因) => {
			if (连接超时定时器 != null) {
				clearTimeout(连接超时定时器)
				连接超时定时器 = null
			}
			if (原因) reject(原因)
			else resolve()
		}

		try {
			新连接 = uni.connectSocket({
				url: 参数.地址,
				protocols: ['mqtt'],
				success: () => { },
				fail: (原因) => {
					连接失败(new Error(原因.errMsg || '无法创建 MQTT WebSocket 连接'))
				},
			})
		} catch (原因) {
			连接失败(原因 instanceof Error ? 原因 : new Error('无法创建 MQTT WebSocket 连接'))
			return
		}
		if (新连接 == null) {
			连接失败(new Error('无法创建 MQTT WebSocket 连接'))
			return
		}
		连接对象 = 新连接
		已连接 = false
		新连接.onOpen(() => {
			if (连接对象 != 新连接) return
			let 连接标志 = 0
			if (参数.清理会话 !== false) 连接标志 |= 0x02
			if (参数.用户名 != null) 连接标志 |= 0x80
			if (参数.密码 != null) 连接标志 |= 0x40
			if (参数.遗嘱 != null) {
				连接标志 |= 0x04
				连接标志 |= (参数.遗嘱.服务质量 & 0x03) << 3
				if (参数.遗嘱.保留) 连接标志 |= 0x20
			}
			const 心跳间隔 = 参数.心跳间隔 ?? 60
			const 内容 = [
				...MQTT字符串字段('MQTT'),
				4,
				连接标志,
				(心跳间隔 >> 8) & 0xFF,
				心跳间隔 & 0xFF,
				...MQTT字符串字段(参数.客户端编号),
			]
			if (参数.遗嘱 != null) {
				内容.push(...MQTT字符串字段(参数.遗嘱.主题), ...MQTT字符串字段(参数.遗嘱.载荷))
			}
			if (参数.用户名 != null) 内容.push(...MQTT字符串字段(参数.用户名))
			if (参数.密码 != null) 内容.push(...MQTT字符串字段(参数.密码))
			发送(MQTT数据包(0x10, 内容))
		})
		新连接.onMessage((事件) => {
			if (连接对象 != 新连接) return
			if (事件.data instanceof ArrayBuffer) 处理数据包(事件.data)
			else if (typeof 事件.data == 'string') {
				处理数据包(new Uint8Array(编码UTF8(事件.data)).buffer)
			}
		})
		新连接.onError(() => {
			if (连接对象 != 新连接) return
			const 原因 = new Error('MQTT WebSocket 连接发生错误')
			触发事件('error', { error: 原因.message })
			连接失败(原因)
		})
		新连接.onClose(() => {
			if (连接对象 != 新连接) return
			连接对象 = null
			已连接 = false
			if (心跳定时器 != null) {
				clearInterval(心跳定时器)
				心跳定时器 = null
			}
			连接失败(new Error('MQTT WebSocket 连接已关闭'))
			触发事件('close')
		})

		连接超时定时器 = setTimeout(() => {
			if (!已连接) {
				连接失败(new Error('MQTT 连接超时'))
				新连接?.close({})
			}
		}, 参数.连接超时 ?? 10000)
	})
}

export function 断开连接() : Promise<void> {
	const 旧连接 = 连接对象
	if (已连接) 发送(new Uint8Array([0xE0, 0]).buffer)
	连接对象 = null
	已连接 = false
	if (心跳定时器 != null) {
		clearInterval(心跳定时器)
		心跳定时器 = null
	}
	if (连接超时定时器 != null) {
		clearTimeout(连接超时定时器)
		连接超时定时器 = null
	}
	连接完成?.(new Error('MQTT 连接已取消'))
	连接完成 = null
	if (旧连接 != null) {
		try {
			旧连接.close({})
		} catch (原因) {
			console.error('关闭 MQTT WebSocket 连接失败', 原因)
		}
	}
	return Promise.resolve()
}

export function 是否已连接() : boolean {
	return 已连接
}
