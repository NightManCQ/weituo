# Changbi MQTT 插件

适用于 UniApp (UTS) 的 MQTT 客户端插件，支持 Android 平台。

**版本**: v1.1.2　|　**平台**: Android ✅ &nbsp; iOS ❌　|　**协议**: MQTT 3.1.1　|　**许可**: MIT

---

## 功能特性

- 支持 TCP、SSL/TLS、WebSocket、WSS 多种连接方式
- 支持 QoS 0 / 1 / 2 三种服务质量等级
- 支持通配符订阅（`+` 单层、`#` 多层）
- 支持遗嘱消息（Will Message）
- 断线自动重连，重连后自动恢复所有订阅
- 离线消息队列：未连接时发送的消息重连后自动补发
- App 后台长连接保活（前台服务 + WorkManager 双层保障）
- 连接质量统计（发送数、接收数、重连次数、平均重连耗时）

---

## 安装

将插件目录复制到项目的 `uni_modules` 目录下即可，依赖会自动配置。

---

## 快速开始

```javascript
import * as mqtt from '@/uni_modules/changbi-mqtt'

// 1. 注册事件（在 connect 之前）
mqtt.on('connect', (data) => {
  console.log('已连接')
  mqtt.subscribe('test/topic', 1)
})

mqtt.on('message', (data) => {
  console.log(`[${data.topic}]`, data.payload)
})

// 2. 建立连接
await mqtt.connect({
  url: 'mqtt://broker.emqx.io:1883',
  clientId: 'myapp_' + Date.now(),
  keepalive: 60
})

// 3. 发布消息
await mqtt.publish('test/topic', 'Hello MQTT', { qos: 1 })

// 4. 断开连接
await mqtt.disconnect()
```

---

## 连接配置

```typescript
mqtt.connect({
  // 必填
  url: string               // 支持 mqtt:// mqtts:// ws:// wss://
  clientId: string

  // 认证
  username?: string
  password?: string

  // 会话
  clean?: boolean           // 默认 true
  connectTimeout?: number   // 单位 ms，默认 30000
  keepalive?: number        // 单位秒，默认 60

  // 自动重连
  reconnect?: boolean       // 默认 true
  reconnectPeriod?: number  // 重连间隔 ms，默认 5000
  maxReconnectAttempts?: number  // 最大重连次数，默认 10

  // SSL
  allowUntrusted?: boolean  // 允许不受信任证书，仅测试用

  // 遗嘱消息
  will?: { topic, payload, qos, retain }
})
```

---

## API

### 连接

| 方法 | 说明 |
|------|------|
| `connect(options)` | 连接到 Broker |
| `disconnect()` | 断开连接 |
| `isConnected()` | 是否已连接，返回 `boolean` |
| `getConnectionState()` | 获取完整连接状态 |

### 订阅

| 方法 | 说明 |
|------|------|
| `subscribe(topic, qos)` | 订阅主题，支持传数组批量订阅 |
| `unsubscribe(topic)` | 取消订阅，支持传数组批量取消 |
| `hasSubscription(topic)` | 是否已订阅某主题 |
| `getSubscriptions()` | 获取所有订阅列表 |
| `getSubscriptionCount()` | 获取订阅数量 |

### 发布

| 方法 | 说明 |
|------|------|
| `publish(topic, message, options?)` | 发布消息，支持字符串和 ArrayBuffer |

### 消息队列

| 方法 | 说明 |
|------|------|
| `getMessageQueueStats()` | 获取队列统计（待发数、已收数等） |
| `clearMessageQueues()` | 清空消息队列 |

### 统计与日志

| 方法 | 说明 |
|------|------|
| `getConnectionQualityStats()` | 获取连接质量统计 |
| `resetConnectionQualityStats()` | 重置统计数据 |
| `setLogLevel(level)` | 设置日志级别（DEBUG/INFO/WARN/ERROR/NONE） |

---

## 事件

```javascript
mqtt.on(event, callback)
mqtt.off(event, callback?)
```

| 事件 | 触发时机 | 主要数据 |
|------|---------|---------|
| `connect` | 连接成功 | `sessionPresent` |
| `message` | 收到消息 | `topic`, `payload`, `qos`, `retain` |
| `reconnect` | 开始重连 | `reconnectAttempts` |
| `close` | 连接关闭 | — |
| `offline` | 重连耗尽/主动断开 | `error` |
| `error` | 发生错误 | `error` |
| `queued` | 消息进入离线队列 | `queuedCount` |
| `queue_flushed` | 离线队列补发完成 | `flushedCount` |

---

## 常用 Broker

| Broker | 地址 |
|--------|------|
| EMQX 公共测试 | `mqtt://broker.emqx.io:1883` |
| Mosquitto 公共测试 | `mqtt://test.mosquitto.org:1883` |
| HiveMQ 公共测试 | `mqtt://broker.hivemq.com:1883` |

---

## 使用条款

本插件最大限度地允许自由使用，但基于开源精神和技术人员的基本道德底线，明确禁止以下行为：

- **禁止换马甲重新上传至任何插件市场**，无论是否收费。换皮上传已经够恶心，若还设置成收费插件，将依法追究法律责任。
- **禁止将插件源码上传至其它任何平台**进行二次分发。
