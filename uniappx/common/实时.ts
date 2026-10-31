import { 获取令牌, 发送请求, 读取当前用户 } from './wangluo'
import { 使用全局变量 } from "../状态/全局.ts"
import type { 用户 } from './leixing'
import type {
	MQTT事件,
	MQTT事件数据,
	MQTT回调,
	MQTT连接参数,
	MQTT订阅参数,
	MQTT订阅消息,
	实时事件,
	实时监听器,
} from './实时.类型'

export type {
	MQTT事件,
	MQTT事件数据,
	MQTT回调,
	MQTT连接参数,
	MQTT遗嘱消息,
	MQTT订阅参数,
	MQTT订阅消息,
	实时事件,
	实时监听器,
} from './实时.类型'

// #ifdef APP-ANDROID
import * as emqxMqtt from '@/uni_modules/changbi-mqtt'
// #endif
// #ifdef H5
import * as emqxMqtt from './webMqtt'
// #endif

const MQTT监听器 = new Map<MQTT事件, Set<MQTT回调>>()
const 主题订阅回调 = new Map<string, Set<(消息: MQTT事件数据) => void>>()
const 实时监听器集合 = new Set<实时监听器>()

let 当前用户 : 用户 | null = null
let 当前身份键 = ''
let 连接中 = false
let 连接代次 = 0
let 正在连接 : Promise<void> | null = null
let 重连定时器 : number | null = null
let 重连次数 = 0

// 消息统一入口：优先派发给按主题注册的回调，否则回退到全局 message 监听
function 分发MQTT消息(数据 : MQTT事件数据) {
	const 主题 = 数据.topic
	const 回调集合 = typeof 主题 == 'string' ? 主题订阅回调.get(主题) : null
	if (回调集合 != null && 回调集合.size > 0) {
		回调集合.forEach((回调) => {
			try {
				回调(数据)
			} catch (原因) {
				console.error('主题订阅回调处理失败', 原因)
			}
		})
		return
	}
	MQTT监听器.get('message')?.forEach((回调) => {
		try {
			回调(数据)
		} catch (原因) {
			console.error('MQTT 消息监听处理失败', 原因)
		}
	})
}

export const emqx = {
	连接(参数 : MQTT连接参数) : Promise<void> {
		if (uni.getSystemInfoSync().platform == 'android') {
			return emqxMqtt.连接({
				url: 参数.地址,
				clientId: 参数.客户端编号,
				username: 参数.用户名,
				password: 参数.密码,
				clean: 参数.清理会话,
				connectTimeout: 参数.连接超时,
				keepalive: 参数.心跳间隔,
				reconnect: 参数.自动重连,
				reconnectPeriod: 参数.重连间隔,
				maxReconnectAttempts: 参数.最大重连次数,
				ssl: 参数.启用SSL,
				certPath: 参数.证书路径,
				allowUntrusted: 参数.允许不受信任证书,
				will: 参数.遗嘱 == null ? null : {
					topic: 参数.遗嘱.主题,
					payload: 参数.遗嘱.载荷,
					qos: 参数.遗嘱.服务质量,
					retain: 参数.遗嘱.保留,
				},
			} as any)
		}
		return emqxMqtt.连接(参数)
	},
	断开连接() : Promise<void> {
		主题订阅回调.clear()
		return emqxMqtt.断开连接()
	},
	是否已连接() : boolean {
		return emqxMqtt.是否已连接()
	},
	订阅(主题 : string | string[], 服务质量 : 0 | 1 | 2 = 1) : Promise<void> {
		const 主题列表 = Array.isArray(主题) ? 主题 : [主题]
		主题列表.forEach((主题项) => {
			let 回调集合 = 主题订阅回调.get(主题项)
			if (回调集合 == null) {
				回调集合 = new Set<(消息: MQTT事件数据) => void>()
				主题订阅回调.set(主题项, 回调集合)
			}
			回调集合.add(解析MQTT消息)
		})
		return 主题列表.length == 0
			? Promise.resolve()
			: emqxMqtt.订阅(主题列表, 服务质量)
	},
	取消订阅(主题 : string | string[] | MQTT订阅参数 | MQTT订阅参数[]) : Promise<void> {
		const 项目列表 = Array.isArray(主题) ? 主题 : [主题]
		const 主题列表 = 项目列表.map((主题项) => typeof 主题项 == 'string' ? 主题项 : 主题项.主题)
		主题列表.forEach((主题项) => 主题订阅回调.delete(主题项))
		return emqxMqtt.取消订阅(主题列表)
	},
	发布(主题 : string, 载荷 : string | ArrayBuffer) : Promise<void> {
		return emqxMqtt.发布(主题, 载荷, { qos: 1 })
	},
	注册事件监听(事件 : MQTT事件, 回调 : MQTT回调) {
		// message 统一由 分发MQTT消息 派发，避免与按主题回调重复触发
		if (事件 != 'message') emqxMqtt.注册事件监听(事件, 回调)
		if (!MQTT监听器.has(事件)) MQTT监听器.set(事件, new Set<MQTT回调>())
		MQTT监听器.get(事件)?.add(回调)
	},
	移除事件监听(事件 : MQTT事件, 回调 ?: MQTT回调) {
		if (事件 != 'message') emqxMqtt.移除事件监听(事件, 回调)
		if (回调 == null) MQTT监听器.delete(事件)
		else MQTT监听器.get(事件)?.delete(回调)
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

// message 事件统一交给 分发MQTT消息，再派发到主题回调或全局 message 监听
emqxMqtt.注册事件监听('message', 分发MQTT消息)

// emqx.注册事件监听('message', 解析MQTT消息)

emqx.注册事件监听('close', () => {
	if (uni.getSystemInfoSync().platform != 'android') 通知关闭时重连()
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
	if (emqx.是否已连接()) return
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
		let 会话列表 : Array<{ MQTT主题 : string }>
		try {
			会话列表 = await 发送请求<Array<{ MQTT主题 : string }>>('/my/message-conversations')
			if (
				本次连接代次 != 连接代次 ||
				当前用户?.用户编号 != 用户信息.用户编号 ||
				获取令牌() != 令牌
			) return
		} catch (原因) {
			console.error('无法取得 MQTT 会话主题', 原因)
			return
		}

		try {
			const 保存地址 = uni.getStorageSync('renwu_mqtt_url') as string || ''
			let 地址 = import.meta.env.VITE_MQTT_URL || 保存地址 || 'ws://localhost:9001/mqtt'
			// EMQX 通过 WebSocket 对外暴露，H5 与 App 原生插件使用不同传输地址
			if (uni.getSystemInfoSync().platform == 'android') {
				地址 = "mqtt" + 地址 + ":1883"
			} else {
				地址 = "ws" + 地址 + ":8084/wss"
			}


			// console.log("-------------------------当前用户", 使用全局变量.系统信息.系统 + 当前用户.用户编号, 用户信息.用户编号, 当前用户.用户编号);
			await emqx.连接({
				地址,
				客户端编号: (使用全局变量.系统信息.系统名 + "-" + 当前用户.用户编号).replace(/\s+/g, ""),
				用户名: 当前用户.用户编号,
				密码: 令牌,
				清理会话: true,
				连接超时: 10000,
				心跳间隔: 60,
				自动重连: true,
				重连间隔: 3000,
				最大重连次数: 10,
			})
			if (本次连接代次 != 连接代次) {
				await emqx.断开连接()
				return
			}
			重连次数 = 0
			const 主题列表 = [...new Set([
				`u/${用户信息.用户编号}`,
				...会话列表.map(会话 => 会话.MQTT主题),
			])]
			await emqx.订阅(主题列表, 1)
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
	主题订阅回调.clear()
	if (重连定时器 != null) {
		clearTimeout(重连定时器)
		重连定时器 = null
	}
	const 旧连接任务 = 正在连接
	正在连接 = null
	if (旧连接任务 != null) await 旧连接任务
	try {
		await emqx.断开连接()
	} catch (原因) {
		console.error('断开 MQTT 实时连接失败', 原因)
	}
}