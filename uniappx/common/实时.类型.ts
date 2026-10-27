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

export type MQTT遗嘱消息 = {
  topic: string
  payload: string
  qos: 0 | 1 | 2
  retain: boolean
}

export type MQTT连接参数 = {
  url: string
  clientId: string
  username?: string | null
  password?: string | null
  clean?: boolean
  connectTimeout?: number
  keepalive?: number
  reconnect?: boolean
  reconnectPeriod?: number
  maxReconnectAttempts?: number
  ssl?: boolean
  certPath?: string | null
  allowUntrusted?: boolean
  will?: MQTT遗嘱消息 | null
}

export type 实时凭据 = {
  消息地址: string
  消息令牌: string
  主题列表: string[]
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
