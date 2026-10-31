export type MQTT事件 =
  | 'connect'
  | 'reconnect'
  | 'disconnect'
  | 'offline'
  | 'close'
  | 'error'
  | 'message'
  | 'queued'
  | 'queue_flushed'
  | 'connection_check'

export type MQTT事件数据 = {
  topic?: string
  payload?: string | ArrayBuffer
  payloadBytes?: ArrayBuffer
  qos?: 0 | 1 | 2
  retain?: boolean
  error?: unknown
  attempt?: number
  reconnectAttempts?: number
  queuedCount?: number
  flushedCount?: number
  isAlive?: boolean
  sessionPresent?: boolean
}

export type MQTT回调 = (数据: MQTT事件数据) => void

export type MQTT订阅消息 = {
  topic?: string
  payload?: string | ArrayBuffer
  qos?: 0 | 1 | 2
  retain?: boolean
}

export type MQTT订阅参数 = {
  主题: string
  服务质量?: 0 | 1 | 2
  成功?: () => void
  失败?: (错误: Error) => void
  回调函数?: (消息: MQTT事件数据) => void
}

export type MQTT遗嘱消息 = {
  主题: string
  载荷: string
  服务质量: 0 | 1 | 2
  保留: boolean
}

export type MQTT连接参数 = {
  地址: string
  客户端编号: string
  用户名?: string | null
  密码?: string | null
  清理会话?: boolean
  连接超时?: number
  心跳间隔?: number
  自动重连?: boolean
  重连间隔?: number
  最大重连次数?: number
  启用SSL?: boolean
  证书路径?: string | null
  允许不受信任证书?: boolean
  遗嘱?: MQTT遗嘱消息 | null
}

export type 实时事件 = {
  主题: string
  事件类型: string
  任务编号?: string
  会话编号?: string
  数据?: Record<string, unknown>
  原始事件: Record<string, unknown>
}

export type 实时监听器 = (事件: 实时事件) => void
