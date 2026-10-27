import { 获取令牌, 发送请求, 读取当前用户 } from './wangluo'
import type { 用户 } from './leixing'

type 实时凭据 = { 消息地址: string; 消息令牌: string; 主题列表: string[] }
export type 实时事件 = {
  主题: string
  事件类型: string
  任务编号?: string
  会话编号?: string
  数据?: Record<string, unknown>
  原始事件: Record<string, unknown>
}
type 实时监听器 = (事件: 实时事件) => void

const 监听器 = new Set<实时监听器>()
let socket: ReturnType<typeof uni.connectSocket> | null = null
let 身份 = ''
let 当前用户: 用户 | null = null
let 重连定时器: number | null = null
let 心跳定时器: number | null = null
let 连接中 = false
let 连接代次 = 0
let 序号 = 1
let 退避次数 = 0

export function 订阅实时事件(回调: 实时监听器): () => void {
  监听器.add(回调)
  return () => { 监听器.delete(回调) }
}

function utf8(文本: string): number[] {
  const 字节: number[] = []
  for (let 索引 = 0; 索引 < 文本.length; 索引++) {
    let 码点 = 文本.charCodeAt(索引)
    if (码点 >= 0xD800 && 码点 <= 0xDBFF && 索引 + 1 < 文本.length) {
      const 低位 = 文本.charCodeAt(索引 + 1)
      if (低位 >= 0xDC00 && 低位 <= 0xDFFF) {
        码点 = ((码点 - 0xD800) << 10) + (低位 - 0xDC00) + 0x10000
        索引++
      }
    }
    if (码点 < 0x80) byte(码点)
    else if (码点 < 0x800) {
      byte(0xC0 | (码点 >> 6))
      byte(0x80 | (码点 & 0x3F))
    } else if (码点 < 0x10000) {
      byte(0xE0 | (码点 >> 12))
      byte(0x80 | ((码点 >> 6) & 0x3F))
      byte(0x80 | (码点 & 0x3F))
    } else {
      byte(0xF0 | (码点 >> 18))
      byte(0x80 | ((码点 >> 12) & 0x3F))
      byte(0x80 | ((码点 >> 6) & 0x3F))
      byte(0x80 | (码点 & 0x3F))
    }
  }
  function byte(值: number) { 字节.push(值) }
  return 字节
}

function 字符串字段(文本: string): number[] {
  const 字节 = utf8(文本)
  return [(字节.length >> 8) & 0xFF, 字节.length & 0xFF, ...字节]
}

function 编码剩余长度(长度: number): number[] {
  const 字节: number[] = []
  do {
    let 当前 = 长度 % 128
    长度 = Math.floor(长度 / 128)
    if (长度 > 0) 当前 |= 0x80
    字节.push(当前)
  } while (长度 > 0)
  return 字节
}

function 数据包(头: number, 内容: number[]): ArrayBuffer {
  const 字节 = new Uint8Array([头, ...编码剩余长度(内容.length), ...内容])
  return 字节.buffer
}

function 发送(内容: ArrayBuffer) {
  socket?.send({ data: 内容 })
}

function 解码文本(字节: Uint8Array): string {
  let 编码 = ''
  for (let 索引 = 0; 索引 < 字节.length; 索引++) {
    编码 += `%${字节[索引].toString(16).padStart(2, '0')}`
  }
  try {
    return decodeURIComponent(编码)
  } catch (原因) {
    console.error('实时消息文本解码失败', 原因)
    return ''
  }
}

function 处理数据包(数据: ArrayBuffer, 凭据: 实时凭据) {
  const 字节 = new Uint8Array(数据)
  if (字节.length < 2) return
  const 类型 = 字节[0] >> 4
  let 游标 = 1
  let 剩余长度 = 0
  let 倍数 = 1
  for (let 次数 = 0; 次数 < 4 && 游标 < 字节.length; 次数++) {
    const 当前 = 字节[游标++]
    剩余长度 += (当前 & 0x7F) * 倍数
    倍数 *= 128
    if ((当前 & 0x80) == 0) break
  }
  const 内容开始位置 = 游标
  if (游标 + 剩余长度 > 字节.length) return

  if (类型 == 2) {
    const 返回码 = 剩余长度 >= 2 ? 字节[游标 + 1] : -1
    if (剩余长度 < 2 || 返回码 != 0) {
      const 拒绝原因 = 返回码 == 1
        ? '协议版本不支持'
        : 返回码 == 2
        ? '客户端标识符无效'
        : 返回码 == 3
        ? '服务不可用'
        : 返回码 == 4
        ? '用户名或密码无效'
        : 返回码 == 5
        ? '未授权'
        : 'CONNACK 报文无效'
      console.error('MQTT broker 拒绝了实时连接', { 返回码, 拒绝原因 })
      socket?.close({})
      return
    }
    const 订阅内容: number[] = [((序号 >> 8) & 0xFF), 序号 & 0xFF]
    序号 = (序号 + 1) & 0xFFFF
    凭据.主题列表.forEach((主题) => { 订阅内容.push(...字符串字段(主题), 1) })
    if (凭据.主题列表.length > 0) 发送(数据包(0x82, 订阅内容))
    return
  }
  if (类型 == 3) {
    if (游标 + 2 > 字节.length) return
    const 主题长度 = (字节[游标] << 8) | 字节[游标 + 1]
    游标 += 2
    if (游标 + 主题长度 > 字节.length) return
    const 主题 = 解码文本(字节.slice(游标, 游标 + 主题长度))
    游标 += 主题长度
    const 服务质量 = (字节[0] >> 1) & 0x03
    let 消息编号 = 0
    if (服务质量 > 0) {
      if (游标 + 2 > 字节.length) return
      消息编号 = (字节[游标] << 8) | 字节[游标 + 1]
      游标 += 2
    }
    if (服务质量 == 1) 发送(数据包(0x40, [(消息编号 >> 8) & 0xFF, 消息编号 & 0xFF]))
    const 文本 = 解码文本(字节.slice(游标, 内容开始位置 + 剩余长度))
    try {
      const 原始 = JSON.parse(文本) as Record<string, unknown>
      const 事件类型 = 原始['事件类型'] ?? 原始['event_type']
      const 任务编号 = 原始['任务编号'] ?? 原始['task_id']
      const 会话编号 = 原始['会话编号'] ?? 原始['conversation_id']
      const 数据内容 = 原始['数据'] ?? 原始['data']
      const 事件: 实时事件 = {
        主题,
        事件类型: typeof 事件类型 == 'string' ? 事件类型 : 'notification.unknown',
        原始事件: 原始,
      }
      if (typeof 任务编号 == 'string') 事件.任务编号 = 任务编号
      if (typeof 会话编号 == 'string') 事件.会话编号 = 会话编号
      if (typeof 数据内容 == 'object' && 数据内容 != null) 事件.数据 = 数据内容 as Record<string, unknown>
      监听器.forEach((回调) => {
        try { 回调(事件) } catch (原因) { console.error('实时消息监听处理失败', 原因) }
      })
    } catch (原因) {
      console.error('无法解析实时 MQTT 消息', 原因)
    }
  }
}

function 重连() {
  if (重连定时器 != null || socket != null || !当前用户 || !获取令牌()) return
  const 延迟 = Math.min(30000, 1000 * (2 ** Math.min(退避次数, 5)))
  退避次数++
  重连定时器 = setTimeout(() => {
    重连定时器 = null
    void 连接()
  }, 延迟)
}

export async function 连接实时消息(用户: 用户 | null = 读取当前用户()) {
  当前用户 = 用户
  if (!用户 || !获取令牌()) {
    断开实时消息()
    return
  }
  const 身份键 = `${用户.用户编号}:${获取令牌()}`
  if (身份键 != 身份) {
    断开实时消息()
    当前用户 = 用户
    身份 = 身份键
  }
  if (socket != null) return
  await 连接()
}

async function 连接() {
  const 用户 = 当前用户
  const 令牌 = 获取令牌()
  if (!用户 || !令牌 || socket != null || 连接中) return
  连接中 = true
  const 本次连接代次 = 连接代次
  try {
    const 凭据 = await 发送请求<实时凭据>('/realtime/credentials', { method: 'POST' })
    if (
      本次连接代次 != 连接代次 ||
      当前用户?.用户编号 != 用户.用户编号 ||
      获取令牌() != 令牌
    ) return
    const 地址覆盖 = uni.getStorageSync('renwu_mqtt_url') as string || ''
    let 地址 = 地址覆盖 || import.meta.env.VITE_MQTT_URL || 凭据.消息地址
    if (地址.startsWith('mqtts://')) 地址 = `wss://${地址.slice(8)}`
    else if (地址.startsWith('mqtt://')) 地址 = `ws://${地址.slice(7)}`
    if ((地址.startsWith('ws://') || 地址.startsWith('wss://')) && !地址.slice(地址.indexOf('://') + 3).includes('/')) 地址 += '/mqtt'

    let 创建中的连接: ReturnType<typeof uni.connectSocket> | null = null
    创建中的连接 = uni.connectSocket({
      url: 地址,
      protocols: ['mqtt'],
      success: () => {},
      fail: (原因) => {
        console.error('无法创建 MQTT WebSocket 连接', 原因)
        setTimeout(() => {
          if (socket == 创建中的连接) socket = null
          重连()
        }, 0)
      },
    })
    const 新连接 = 创建中的连接
    if (新连接 == null) throw new Error('无法创建 MQTT WebSocket 连接')
    socket = 新连接
    新连接.onOpen(() => {
      退避次数 = 0
      const 内容 = [
        ...字符串字段('MQTT'),
        4,
        0xC2,
        0,
        60,
        ...字符串字段(用户.登录名),
        ...字符串字段(用户.用户编号),
        ...字符串字段(凭据.消息令牌),
      ]
      发送(数据包(0x10, 内容))
      if (心跳定时器 != null) clearInterval(心跳定时器)
      心跳定时器 = setInterval(() => 发送(new Uint8Array([0xC0, 0]).buffer), 45000)
    })
    新连接.onMessage((事件) => {
      if (事件.data instanceof ArrayBuffer) 处理数据包(事件.data, 凭据)
      else if (typeof 事件.data == 'string') {
        const 编码后 = utf8(事件.data)
        处理数据包(new Uint8Array(编码后).buffer, 凭据)
      }
    })
    新连接.onError((事件) => {
      console.error('MQTT 实时消息连接失败', 事件)
      新连接.close({})
    })
    新连接.onClose(() => {
      if (socket != 新连接) return
      socket = null
      if (心跳定时器 != null) {
        clearInterval(心跳定时器)
        心跳定时器 = null
      }
      重连()
    })
  } catch (原因) {
    console.error('无法取得 MQTT 实时消息凭据', 原因)
    重连()
  } finally {
    if (本次连接代次 == 连接代次) 连接中 = false
  }
}

export function 断开实时消息() {
  连接代次++
  连接中 = false
  当前用户 = null
  身份 = ''
  if (重连定时器 != null) {
    clearTimeout(重连定时器)
    重连定时器 = null
  }
  if (心跳定时器 != null) {
    clearInterval(心跳定时器)
    心跳定时器 = null
  }
  const 连接任务 = socket
  socket = null
  if (连接任务 != null) 连接任务.close({})
}
