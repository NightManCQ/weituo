import type { 任务, 任务位置, 用户 } from './leixing'

const 默认API地址 = import.meta.env.VITE_API_URL || 凭据.消息地址

type 请求选项 = {
	method ?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'
	data ?: UTSJSONObject
	headers ?: Record<string, string>
}

export class 请求错误 extends Error {
	状态码 : number
	constructor(消息 : string, 状态码 : number) {
		super(消息)
		this.name = '请求错误'
		this.状态码 = 状态码
	}
}

export function 获取令牌() : string {
	return uni.getStorageSync('renwu_token') as string || ''
}

export function 获取API地址() : string {
	const 配置地址 = uni.getStorageSync('renwu_api_base') as string || ''
	return (配置地址 || 默认API地址).replace(/\/+$/, '')
}

export function 读取当前用户() : 用户 | null {
	const 内容 = uni.getStorageSync('renwu_user')
	if (typeof 内容 != 'object' || 内容 == null) return null
	const 用户信息 = 内容 as 用户
	if (
		typeof 用户信息.用户编号 == 'string' &&
		typeof 用户信息.登录名 == 'string' &&
		typeof 用户信息.昵称 == 'string'
	) return 用户信息
	const 旧资料 = 内容 as UTSJSONObject
	if (
		typeof 旧资料['user_id'] == 'string' &&
		typeof 旧资料['login_name'] == 'string' &&
		typeof 旧资料['nickname'] == 'string'
	) {
		const 迁移资料 : 用户 = {
			用户编号: 旧资料['user_id'] as string,
			登录名: 旧资料['login_name'] as string,
			昵称: 旧资料['nickname'] as string,
		}
		uni.setStorageSync('renwu_user', 迁移资料)
		return 迁移资料
	}
	uni.removeStorageSync('renwu_user')
	uni.removeStorageSync('renwu_token')
	return null
}

export function 读取委托位置() : 任务位置 | null {
	const 内容 = uni.getStorageSync('renwu_location')
	if (typeof 内容 != 'object' || 内容 == null) return null
	const 位置 = 内容 as 任务位置
	if (typeof 位置.经度 == 'number' && typeof 位置.纬度 == 'number') return 位置
	uni.removeStorageSync('renwu_location')
	return null
}

export function 发送请求<T>(路径 : string, 选项 : 请求选项 = {}) : Promise<T> {
	return new Promise<T>((resolve, reject) => {
		const 请求头 : UTSJSONObject = { 'Content-Type': 'application/json' }
		if (选项.headers) {
			Object.keys(选项.headers).forEach((键) => {
				请求头[键] = 选项.headers![键]
			})
		}
		const 令牌 = 获取令牌()
		if (令牌.length > 0) 请求头['Authorization'] = `Bearer ${令牌}`
		// console.log("???????????", `${获取API地址()}${路径}`);
		uni.request({
			url: `${获取API地址()}${路径}`,
			method: 选项.method || 'GET',
			data: 选项.data || {},
			header: 请求头,
			timeout: 15000,
			success: (响应) => {
				const 内容 = 响应.data as UTSJSONObject | null
				if (响应.statusCode < 200 || 响应.statusCode >= 300) {
					if (响应.statusCode == 401) {
						uni.removeStorageSync('renwu_token')
						uni.removeStorageSync('renwu_user')
					}
					const 消息 = 响应.statusCode == 401
						? '登录状态已失效，请重新登录'
						: 内容 != null && typeof 内容['消息'] == 'string'
							? 内容['消息'] as string
							: `请求失败 (${响应.statusCode})`
					reject(new 请求错误(消息, 响应.statusCode))
					return
				}
				if (内容 == null || typeof 内容['数据'] == 'undefined' || typeof 内容['状态码'] != 'number') {
					reject(new Error('服务器返回了无法识别的响应结构'))
					return
				}
				if (内容['状态码'] != 响应.statusCode) {
					reject(new Error('服务器响应状态码与响应内容不一致'))
					return
				}
				resolve(内容['数据'] as T)
			},
			fail: (原因) => reject(new Error(
				原因.errMsg == 'request:fail'
					? '请求服务器失败，请确认 API 地址和服务状态'
					: 原因.errMsg || '网络请求失败，请检查 API 地址和网络',
			)),
		})
	})
}

export function 格式化时间(值 : string) : string {
	if (!值) return ''
	const 日期 = new Date(值)
	if (Number.isNaN(日期.getTime())) return 值
	return 日期.toLocaleString('zh-CN', { dateStyle: 'medium', timeStyle: 'short' })
}

export function 创建编号() : string {
	const 随机 = `${Math.random().toString(36).slice(2)}${Math.random().toString(36).slice(2)}`
	return `${Date.now()}-${随机.slice(0, 20)}`
}

export async function 发布委托(
	标题 : string,
	描述 : string,
	截止时间 : string,
	报酬积分 : number,
	位置 : 任务位置 | null,
) : Promise<任务> {
	const 幂等键 = 创建编号()
	return await 发送请求<任务>('/tasks', {
		method: 'POST',
		headers: { 'Idempotency-Key': 幂等键 },
		data: {
			title: 标题,
			description: 描述,
			deadline: 截止时间 ? new Date(截止时间).toISOString() : null,
			reward_points: 报酬积分,
			location: 位置 == null ? null : {
				longitude: 位置.经度,
				latitude: 位置.纬度,
				country: 位置.国,
				province: 位置.省,
				city: 位置.市,
				county: 位置.县,
				town: 位置.镇,
				village: 位置.村,
				address: 位置.地址,
			},
		} as UTSJSONObject,
	})
}

export async function 加载积分() : Promise<积分账户> {
	return await 发送请求<积分账户>('/my/points')
}