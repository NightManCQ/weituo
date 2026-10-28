import { 获取令牌, 发送请求, 读取当前用户 } from './wangluo'
import { 使用全局变量 } from "../状态/全局.ts"
import type { 用户 } from './leixing'
import type {
	MQTT事件,
	MQTT事件数据,
	MQTT回调,
	MQTT连接参数,
	实时凭据,
	实时事件,
	实时监听器,
} from './实时.类型'

export type {
	MQTT事件,
	MQTT事件数据,
	MQTT回调,
	MQTT连接参数,
	MQTT遗嘱消息,
	实时凭据,
	实时事件,
	实时监听器,
} from './实时.类型'

// #ifdef APP-ANDROID
import * as 原生Mqtt from '@/uni_modules/changbi-mqtt'
// #endif

const MQTT监听器 = new Map<MQTT事件, Set<MQTT回调>>()
const 实时监听器集合 = new Set<实时监听器>()

let Web连接 : ReturnType<typeof uni.connectSocket> | null = null
let Web已连接 = false
let Web消息编号 = 1
let Web心跳定时器 : number | null = null
let Web连接超时定时器 : number | null = null
let Web连接完成 : ((原因 ?: Error) => void) | null = null
let 当前用户 : 用户 | null = null
let 当前身份键 = ''
let 连接中 = false
let 连接代次 = 0
let 正在连接 : Promise<void> | null = null
let 重连定时器 : number | null = null
let 重连次数 = 0
let 凭据缓存身份键 = ''
let 实时凭据请求 : Promise<实时凭据> | null = null

function 通知MQTT监听器(事件 : MQTT事件, 数据 : MQTT事件数据 = {}) {
	MQTT监听器.get(事件)?.forEach((回调) => {
		try {
			回调(数据)
		} catch (原因) {
			console.error('MQTT 事件监听处理失败', 原因)
		}
	})
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

function 获取Web消息编号() : number {
	const 当前编号 = Web消息编号
	Web消息编号 = Web消息编号 >= 65535 ? 1 : Web消息编号 + 1
	return 当前编号
}

function Web发送(数据 : ArrayBuffer) {
	Web连接?.send({ data: 数据 })
}

function Web订阅(主题 : string | string[], qos : 0 | 1 | 2 = 0) : Promise<void> {
	if (!Web已连接) return Promise.reject(new Error('MQTT 尚未连接'))
	const 主题列表 = Array.isArray(主题) ? 主题 : [主题]

	const 消息编号 = 获取Web消息编号()
	const 内容 = [(消息编号 >> 8) & 0xFF, 消息编号 & 0xFF]
	主题列表.forEach((项) => 内容.push(...MQTT字符串字段(项), qos))
	Web发送(MQTT数据包(0x82, 内容))
	return Promise.resolve()
}

function Web取消订阅(主题 : string | string[]) : Promise<void> {
	if (!Web已连接) return Promise.reject(new Error('MQTT 尚未连接'))
	const 主题列表 = Array.isArray(主题) ? 主题 : [主题]

	const 消息编号 = 获取Web消息编号()
	const 内容 = [(消息编号 >> 8) & 0xFF, 消息编号 & 0xFF]
	主题列表.forEach((项) => 内容.push(...MQTT字符串字段(项)))
	Web发送(MQTT数据包(0xA2, 内容))
	return Promise.resolve()
}

function Web处理数据包(数据 : ArrayBuffer) {
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
				Web连接完成?.(原因)
				Web连接完成 = null
				通知MQTT监听器('error', { error: 原因.message })
				Web连接?.close({})
				return
			}
			Web已连接 = true
			重连次数 = 0
			if (Web心跳定时器 != null) clearInterval(Web心跳定时器)
			Web心跳定时器 = setInterval(() => Web发送(new Uint8Array([0xC0, 0]).buffer), 45000)
			Web连接完成?.()
			Web连接完成 = null
			通知MQTT监听器('connect')
		} else if (类型 == 3) {
			if (剩余长度 < 2) return
			const 主题长度 = (字节[内容开始位置] << 8) | 字节[内容开始位置 + 1]
			let 内容游标 = 内容开始位置 + 2
			if (内容游标 + 主题长度 > 内容结束位置) return
			const 主题 = 解码UTF8(字节.slice(内容游标, 内容游标 + 主题长度))
			内容游标 += 主题长度
			const 服务质量 = (头 >> 1) & 0x03
			let 消息编号 = 0
			if (服务质量 > 0) {
				if (内容游标 + 2 > 内容结束位置) return
				消息编号 = (字节[内容游标] << 8) | 字节[内容游标 + 1]
				内容游标 += 2
			}
			if (服务质量 == 1) {
				Web发送(MQTT数据包(0x40, [(消息编号 >> 8) & 0xFF, 消息编号 & 0xFF]))
			}
			通知MQTT监听器('message', {
				topic: 主题,
				payload: 解码UTF8(字节.slice(内容游标, 内容结束位置)),
			})
		}
		游标 = 内容结束位置
	}
}

function Web连接MQTT(参数 : MQTT连接参数) : Promise<void> {
	if (Web连接 != null) return Promise.reject(new Error('Web MQTT 连接正在使用中'))

	return new Promise<void>((resolve, reject) => {
		let 新连接 : ReturnType<typeof uni.connectSocket> | null = null
		const 连接失败 = (原因 : Error) => {
			if (Web连接完成 == null) return
			Web连接完成 = null
			if (Web连接超时定时器 != null) {
				clearTimeout(Web连接超时定时器)
				Web连接超时定时器 = null
			}
			reject(原因)
		}
		Web连接完成 = (原因) => {
			if (Web连接超时定时器 != null) {
				clearTimeout(Web连接超时定时器)
				Web连接超时定时器 = null
			}
			if (原因) reject(原因)
			else resolve()
		}

		try {
			新连接 = uni.connectSocket({
				url: 参数.url,
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
		Web连接 = 新连接
		Web已连接 = false
		新连接.onOpen(() => {
			if (Web连接 != 新连接) return
			const 内容 = [
				...MQTT字符串字段('MQTT'),
				4,
				0xC2,
				0,
				参数.keepalive ?? 60,
				...MQTT字符串字段(参数.clientId),
				...MQTT字符串字段(参数.username ?? ''),
				...MQTT字符串字段(参数.password ?? ''),
			]
			Web发送(MQTT数据包(0x10, 内容))
		})
		新连接.onMessage((事件) => {
			if (Web连接 != 新连接) return
			if (事件.data instanceof ArrayBuffer) Web处理数据包(事件.data)
			else if (typeof 事件.data == 'string') {
				Web处理数据包(new Uint8Array(编码UTF8(事件.data)).buffer)
			}
		})
		新连接.onError((事件) => {
			if (Web连接 != 新连接) return
			const 原因 = new Error('MQTT WebSocket 连接发生错误')
			通知MQTT监听器('error', { error: 原因.message })
			连接失败(原因)
		})
		新连接.onClose(() => {
			if (Web连接 != 新连接) return
			Web连接 = null
			Web已连接 = false
			if (Web心跳定时器 != null) {
				clearInterval(Web心跳定时器)
				Web心跳定时器 = null
			}
			连接失败(new Error('MQTT WebSocket 连接已关闭'))
			通知MQTT监听器('close')
		})

		Web连接超时定时器 = setTimeout(() => {
			if (!Web已连接) {
				连接失败(new Error('MQTT 连接超时'))
				新连接?.close({})
			}
		}, 参数.connectTimeout ?? 10000)
	})
}

export const emqx = {
	connect(参数 : MQTT连接参数) : Promise<void> {

		// #ifdef APP-ANDROID
		console.log("________________________________连接参数", 参数);
		return 原生Mqtt.connect(参数)
		// #endif
		// #ifdef H5

		// 参数.clientId = 'h5-' + 参数.clientId
		return Web连接MQTT(参数)
		// #endif
		return Promise.reject(new Error('当前平台尚未接入 MQTT 传输实现'))
	},
	disconnect() : Promise<void> {
		// #ifdef APP-ANDROID
		return 原生Mqtt.disconnect()
		// #endif
		// #ifdef H5
		const 旧连接 = Web连接
		Web连接 = null
		Web已连接 = false
		if (Web心跳定时器 != null) {
			clearInterval(Web心跳定时器)
			Web心跳定时器 = null
		}
		if (旧连接 != null) 旧连接.close({})
		return Promise.resolve()
		// #endif
		return Promise.resolve()
	},
	isConnected() : boolean {
		// #ifdef APP-ANDROID
		return 原生Mqtt.isConnected()
		// #endif
		// #ifdef H5
		return Web已连接
		// #endif
		return false
	},
	subscribe(主题 : string | string[], qos : 0 | 1 | 2 = 0) : Promise<void> {
		// #ifdef APP-ANDROID
		return 原生Mqtt.subscribe(主题, qos)
		// #endif
		// #ifdef H5
		return Web订阅(主题, qos)
		// #endif
		return Promise.reject(new Error('当前平台尚未接入 MQTT 订阅实现'))
	},
	unsubscribe(主题 : string | string[]) : Promise<void> {
		// #ifdef APP-ANDROID
		return 原生Mqtt.unsubscribe(主题)
		// #endif
		// #ifdef H5
		return Web取消订阅(主题)
		// #endif
		return Promise.resolve()
	},
	publish(主题 : string, payload : string | ArrayBuffer) : Promise<void> {
		// #ifdef APP-ANDROID
		return 原生Mqtt.publish(主题, payload, { qos: 1 })
		// #endif
		// #ifdef H5
		if (!Web已连接) return Promise.reject(new Error('MQTT 尚未连接'))
		const 内容字节 = typeof payload == 'string' ? 编码UTF8(payload) : Array.from(new Uint8Array(payload))
		const 消息内容 = [...MQTT字符串字段(主题), ...内容字节]
		Web发送(MQTT数据包(0x30, 消息内容))
		return Promise.resolve()
		// #endif
		return Promise.reject(new Error('当前平台尚未接入 MQTT 发布实现'))
	},
	on(事件 : MQTT事件, 回调 : MQTT回调) {
		// #ifdef APP-ANDROID
		原生Mqtt.on(事件, 回调)
		// #endif
		// #ifdef H5
		if (!MQTT监听器.has(事件)) MQTT监听器.set(事件, new Set<MQTT回调>())
		MQTT监听器.get(事件)?.add(回调)
		// #endif
	},
	off(事件 : MQTT事件, 回调 ?: MQTT回调) {
		// #ifdef APP-ANDROID
		原生Mqtt.off(事件, 回调)
		// #endif
		// #ifdef H5
		if (回调 == null) MQTT监听器.delete(事件)
		else MQTT监听器.get(事件)?.delete(回调)
		// #endif
	},
}

function 分发实时事件(事件 : 实时事件) {
	实时监听器集合.forEach((回调) => {
		try {
			回调(事件)
		} catch (原因) {
			console.error('实时消息监听处理失败', 原因)
		}
	})
}

function 解析MQTT消息(数据 : MQTT事件数据) {
	console.log("'================得到数据", 数据);
	if (typeof 数据.topic != 'string' || typeof 数据.payload != 'string') return
	try {
		const 内容 : unknown = JSON.parse(数据.payload)
		if (typeof 内容 != 'object' || 内容 == null || Array.isArray(内容)) {
			throw new Error('实时消息必须是 JSON 对象')
		}
		const 原始事件 = 内容 as Record<string, unknown>
		const 事件类型 = 原始事件['事件类型'] ?? 原始事件['event_type']
		const 任务编号 = 原始事件['任务编号'] ?? 原始事件['task_id']
		const 会话编号 = 原始事件['会话编号'] ?? 原始事件['conversation_id']
		const 事件数据 = 原始事件['数据'] ?? 原始事件['data']
		if (事件类型 !== undefined && typeof 事件类型 != 'string') throw new Error('实时消息事件类型无效')
		if (任务编号 !== undefined && typeof 任务编号 != 'string') throw new Error('实时消息任务编号无效')
		if (会话编号 !== undefined && typeof 会话编号 != 'string') throw new Error('实时消息会话编号无效')
		if (
			事件数据 !== undefined &&
			(typeof 事件数据 != 'object' || 事件数据 == null || Array.isArray(事件数据))
		) throw new Error('实时消息数据必须是对象')

		const 实时事件对象 : 实时事件 = {
			主题: 数据.topic,
			事件类型: typeof 事件类型 == 'string' ? 事件类型 : 'notification.unknown',
			原始事件,
		}
		if (typeof 任务编号 == 'string') 实时事件对象.任务编号 = 任务编号
		if (typeof 会话编号 == 'string') 实时事件对象.会话编号 = 会话编号
		if (typeof 事件数据 == 'object' && 事件数据 != null) {
			实时事件对象.数据 = 事件数据 as Record<string, unknown>
		}
		分发实时事件(实时事件对象)
	} catch (原因) {
		console.error('无法解析 MQTT 实时消息', 原因)
	}
}

emqx.on('message', 解析MQTT消息)

export function 订阅实时事件(回调 : 实时监听器) : () => void {
	实时监听器集合.add(回调)
	return () => { 实时监听器集合.delete(回调) }
}

function 通知关闭时重连() {
	if (重连定时器 != null || 当前用户 == null || !获取令牌()) return
	const 延迟 = Math.min(30000, 1000 * (2 ** Math.min(重连次数, 5)))
	重连次数++
	重连定时器 = setTimeout(() => {
		重连定时器 = null
		void 连接()
	}, 延迟)
}

emqx.on('close', () => {
	// #ifdef H5
	通知关闭时重连()
	// #endif
})

export async function 连接实时消息(用户 : 用户 | null = 读取当前用户()) {
	当前用户 = 用户
	if (!用户 || !获取令牌()) {
		await 断开实时消息()
		return
	}

	const 新身份键 = `${用户.用户编号}:${获取令牌()}`
	if (新身份键 != 当前身份键) {
		await 断开实时消息()
		当前用户 = 用户
		当前身份键 = 新身份键
	}
	if (emqx.isConnected()) return
	if (正在连接 != null) return await 正在连接
	await 连接()
}

async function 连接() {
	const 用户信息 = 当前用户
	const 令牌 = 获取令牌()
	if (!用户信息 || !令牌 || 连接中) return
	连接中 = true
	const 本次连接代次 = 连接代次
	const 连接任务 = (async () => {
		let 凭据 : 实时凭据
		try {
			if (凭据缓存身份键 != 当前身份键 || 实时凭据请求 == null) {
				凭据缓存身份键 = 当前身份键
				实时凭据请求 = 发送请求<实时凭据>('/realtime/credentials', { method: 'POST' })
			}
			凭据 = await 实时凭据请求
			if (
				本次连接代次 != 连接代次 ||
				当前用户?.用户编号 != 用户信息.用户编号 ||
				获取令牌() != 令牌
			) return
		} catch (原因) {
			console.error('无法取得 MQTT 实时消息凭据', 原因)
			return
		}

		try {
			let 地址 = import.meta.env.VITE_MQTT_URL || 凭据.消息地址
			// EMQX 通过 WebSocket 对外暴露，H5 与 App 原生插件统一使用 ws:// 协议
			// if (地址.startsWith('://')) 地址 = 'ws' + 地址
			// #ifdef H5
			地址 = "ws" + 地址 + ":8084/wss"
			// console.log("-------------------------在h5平台编译", 地址);
			// #endif

			// #ifdef APP-ANDROID
			地址 = "mqtt" + 地址 + ":1883"
			// console.log("-------------------------在h5平台编译", 地址);
			// #endif


			// console.log("-------------------------当前用户", 使用全局变量.系统信息.系统 + 当前用户.用户编号, 用户信息.用户编号, 当前用户.用户编号);
			await emqx.connect({
				url: 地址,
				clientId: (使用全局变量.系统信息.系统名 + "-" + 当前用户.用户编号).replace(/\s+/g, ""),
				username: 当前用户.登录名,
				password: 凭据.消息令牌,
				clean: true,
				connectTimeout: 10000,
				keepalive: 60,
				reconnect: true,
				reconnectPeriod: 3000,
				maxReconnectAttempts: 10,
			})
			if (本次连接代次 != 连接代次) {
				await emqx.disconnect()
				return
			}
			重连次数 = 0
			if (凭据.主题列表.length > 0) await emqx.subscribe(凭据.主题列表, 1)
			console.log("---------主题列表", 凭据);
		} catch (原因) {
			console.error('无法连接 MQTT 实时消息服务', 原因)
			if (本次连接代次 == 连接代次) 通知关闭时重连()
		}
	})()
	正在连接 = 连接任务
	try {
		await 连接任务
	} finally {
		if (正在连接 == 连接任务) {
			正在连接 = null
			连接中 = false
		}
	}
}

export async function 断开实时消息() {
	连接代次++
	连接中 = false
	当前用户 = null
	当前身份键 = ''
	重连次数 = 0
	凭据缓存身份键 = ''
	实时凭据请求 = null
	if (重连定时器 != null) {
		clearTimeout(重连定时器)
		重连定时器 = null
	}
	if (Web心跳定时器 != null) {
		clearInterval(Web心跳定时器)
		Web心跳定时器 = null
	}
	if (Web连接超时定时器 != null) {
		clearTimeout(Web连接超时定时器)
		Web连接超时定时器 = null
	}
	const 旧连接任务 = 正在连接
	正在连接 = null
	Web连接完成?.(new Error('MQTT 连接已取消'))
	Web连接完成 = null
	if (旧连接任务 != null) await 旧连接任务
	if (emqx.isConnected() || Web连接 != null) {
		try {
			await emqx.disconnect()
		} catch (原因) {
			console.error('断开 MQTT 实时连接失败', 原因)
		}
	}
}