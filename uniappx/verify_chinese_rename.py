# -*- coding: utf-8 -*-
"""校验 changbi-mqtt/app-android/index.uts 中文化改名的完整性与契约保真。"""
import os, re, sys

BASE = r"C:\Users\cq\Documents\gong_zuo\jiaz_kz\renwu_xitong\uniappx\uni_modules\changbi-mqtt\utssdk\app-android"
索引文件 = os.path.join(BASE, "index.uts")
实时文件 = r"C:\Users\cq\Documents\gong_zuo\jiaz_kz\renwu_xitong\uniappx\common\实时.ts"

文本 = open(索引文件, encoding="utf-8").read()

# 去除字符串字面量（单引号/双引号/模板串），避免字符串内容干扰标识符检测
去串 = re.sub(r"'(?:[^'\\]|\\.)*'|\"(?:[^\"\\]|\\.)*\"|`(?:[^`\\]|\\.)*`", "", 文本)
# 去除注释
去串注释 = re.sub(r"/\*.*?\*/", "", 去串, flags=re.S)
去串注释 = re.sub(r"(?m)(?<!:)//.*?$", "", 去串注释)

错误 = []

# 1) 旧英文局部标识符不得再以标识符形式出现
# （注：sessionPresent/isAlive/topic/retain/clean/keepalive/allowUntrusted/
#   flushedCount/username/password 是外部类型字段，属契约，不列入旧名）
旧名 = [
    "mqttHelper", "eventListeners", "logLevel", "subscriptionManager",
    "messageHandler", "stateManager", "connectionManager",
    "connectCallbackRef", "messageCallbackRef", "closeCallbackRef",
    "connectionLostCallbackRef", "userEventCallbacks", "qualityStats",
    "disconnectedAtMs", "totalReconnectMs", "initMqttHelper", "emitEvent",
    "logMessage", "resolve", "reject",
    "payloadStr", "willTopic", "willPayload", "willQos", "willRetain",
    "topics", "completed", "hasError", "pending", "nativePayload",
    "byteArray", "queuedData", "snapshot",
    "listeners", "index", "subs", "result", "config",
    "context", "appContext", "errMsg", "errorMsg", "elapsed",
]
for 名 in 旧名:
    if re.search(r"\b" + 名 + r"\b", 去串注释):
        错误.append(f"旧英文标识符仍残留: {名}")

# 2) 外部契约名必须保留（import 类型、Kotlin 类、公共方法、字段、事件名）
外部契约 = [
    "MqttHelper", "MqttForegroundService", "UTSAndroid", "UTSJSONObject",
    "SubscriptionManager", "MessageHandler", "StateManager", "ConnectionManager",
    "ConnectionStatus", "ReconnectConfig", "ConnectionOptions", "PublishOptions",
    "MQTTEvent", "EventCallback", "EventData", "ConnectionState", "LogLevel",
    "QoS", "ConnectionQualityStats", "ByteArray", "Uint8Array",
    "getInstance", "getAppContext", "setStateChangeCallback", "getSubscriptions",
    "setConnectCallback", "setMessageCallback", "setCloseCallback",
    "setConnectionCheckCallback", "setConnectionLostCallback",
    "setReconnectAttemptsCallback", "setReconnecting", "setReconnectCallback",
    "getConnectionOptions", "onReconnectSuccess", "resetReconnectAttempts",
    "onReconnectFailure", "setMaxAttemptsCallback", "setConnected",
    "setDisconnected", "setDisconnecting", "setConnecting", "handleIncomingMessage",
    "enableReconnect", "disableReconnect", "saveConnectionOptions",
    "setReconnectConfig", "addSubscription", "removeSubscription",
    "hasSubscription", "clearSubscriptions", "handleConnectionLost",
    "queueMessage", "getPendingMessages", "getTotalQueued", "clearPendingMessages",
    "incrementFlushed", "getTotalFlushed", "getQueueStats", "clearReceivedMessages",
    "getSubscriptionCount", "getConnectionState", ".start(", ".stop(",
    ".connect(", ".disconnect(", ".isConnected()", ".publish(", ".subscribe(",
    ".unsubscribe(", ".toInt()", ".toByte()", ".length", ".slice()",
    "totalPublished", "totalReceived", "totalQueued", "totalQueueFlushed",
    "totalReconnects", "lastConnectedAt", "lastDisconnectedAt", "avgReconnectMs",
    "sessionPresent", "lastError", "reconnectAttempts", "connectedAt",
    "reconnectPeriod", "maxReconnectAttempts", "connectTimeout", "clientId",
    "keepalive", "queuedCount", "flushedCount", "queue_flushed",
    "'connect'", "'close'", "'offline'", "'reconnect'", "'message'", "'error'",
    "'connection_check'", "'queued'",
]
for 名 in 外部契约:
    if 名 not in 文本:
        错误.append(f"外部契约名丢失: {名}")

# 3) 本文件导出名
导出名 = [
    "连接", "断开连接", "是否已连接", "订阅", "取消订阅", "发布",
    "补发消息队列", "注册事件监听", "移除事件监听", "获取连接状态",
    "设置日志级别", "获取全部订阅", "获取订阅数量", "是否已订阅",
    "获取消息队列统计", "清空消息队列", "获取连接质量统计", "重置连接质量统计",
]
for 名 in 导出名:
    if not re.search(r"\bexport\s+function\s+" + 名, 文本):
        错误.append(f"导出函数缺失: {名}")

# 4) 调用方 实时.ts
实时文本 = open(实时文件, encoding="utf-8").read()
调用 = set(re.findall(r"原生Mqtt\.([\u4e00-\u9fa5]+)\(", 实时文本))
期望 = {"连接", "断开连接", "是否已连接", "订阅", "取消订阅", "发布", "注册事件监听", "移除事件监听"}
if 调用 != 期望:
    错误.append(f"实时.ts 调用集合不符: 实际={sorted(调用)} 期望={sorted(期望)}")
if re.search(r"原生Mqtt\.(connect|disconnect|isConnected|subscribe|unsubscribe|publish|on|off)\(", 实时文本):
    错误.append("实时.ts 仍残留旧英文调用")

print(f"错误数: {len(错误)}")
for e in 错误:
    print("  [E]", e)
sys.exit(1 if 错误 else 0)
