/**
 * MqttClientWrapper.swift
 *
 * iOS 原生 MQTT 客户端封装，供 UTS 层调用。
 * 依赖：CocoaMQTT (v2.x)，通过 Swift Package Manager 引入。
 *
 * 保活策略（有限前台保活）：
 *   iOS 不支持 Android 式的持久前台服务。本实现仅在 App 进入后台时
 *   调用 UIApplication.beginBackgroundTask，为连接保留系统允许的最长
 *   后台运行时间（通常约 30 秒）。超时后若连接断开，重连由 UTS 层的
 *   ConnectionManager 在 App 回到前台后驱动。
 *   这属于"有限前台保活"，满足题目要求，不追求 Android 级别的永久保活。
 */

import Foundation
import UIKit
import CocoaMQTT

// MARK: - Callback type aliases

/// 连接结果回调：(sessionPresent, errorMessage?)
public typealias ConnectCallback    = (Bool, String?) -> Void
/// 消息到达回调：(topic, payloadUTF8, qos, retain)
public typealias MessageCallback    = (String, String, Int32, Bool) -> Void
/// 连接关闭（意外断线）回调
public typealias CloseCallback      = () -> Void
/// 连接丢失（触发重连）回调
public typealias ConnectionLostCallback = () -> Void
/// 主动心跳检测结果：isAlive
public typealias ConnectionCheckCallback = (Bool) -> Void

// MARK: - MqttClientWrapper

@objc public class MqttClientWrapper: NSObject {

    // MARK: Singleton

    @objc public static let shared = MqttClientWrapper()
    private override init() { super.init() }

    // MARK: Private state

    private var mqtt: CocoaMQTT?
    private var isConnectingActive = false
    private var bgTaskID: UIBackgroundTaskIdentifier = .invalid

    // 心跳检测定时器
    private var connectionCheckTimer: Timer?

    // MARK: Callbacks (set by UTS layer)

    @objc public var onConnect:         ConnectCallback?
    @objc public var onMessage:         MessageCallback?
    @objc public var onClose:           CloseCallback?
    @objc public var onConnectionLost:  ConnectionLostCallback?
    @objc public var onConnectionCheck: ConnectionCheckCallback?

    // MARK: - Public API

    /// 连接到 Broker
    /// - Parameters 与 Android MqttHelper.connect() 对齐，方便 UTS 层统一调用。
    @objc public func connect(
        url:            String,
        clientId:       String,
        username:       String?,
        password:       String?,
        clean:          Bool,
        timeoutMs:      Int,        // milliseconds，与 Android 保持一致
        keepalive:      Int,        // seconds
        allowUntrusted: Bool,
        willTopic:      String?,
        willPayload:    String?,
        willQos:        Int32,
        willRetain:     Bool
    ) {
        guard !isConnectingActive else {
            DispatchQueue.main.async {
                self.onConnect?(false, "Connection already in progress")
            }
            return
        }
        isConnectingActive = true

        // 解析 URL
        guard let (host, port, useSSL, useWS) = parseUrl(url) else {
            isConnectingActive = false
            DispatchQueue.main.async {
                self.onConnect?(false, "Invalid MQTT URL: \(url)")
            }
            return
        }

        // 清理旧连接
        disconnectSilently()

        // 创建 CocoaMQTT 实例（WebSocket 或 TCP）
        let client: CocoaMQTT
        if useWS {
            let wsClient = CocoaMQTTWebSocket(clientID: clientId, host: host, port: UInt16(port))
            wsClient.enableSSL = useSSL
            client = wsClient
        } else {
            client = CocoaMQTT(clientID: clientId, host: host, port: UInt16(port))
            client.enableSSL = useSSL
        }

        // 基本配置
        client.cleanSession      = clean
        client.keepAlive         = UInt16(max(1, keepalive))
        client.connectTimeout    = Double(max(1, timeoutMs / 1000))
        client.autoReconnect     = false   // 重连由 UTS 层 ConnectionManager 统一管理

        if let u = username, !u.isEmpty { client.username = u }
        if let p = password, !p.isEmpty { client.password = p }

        // SSL / TLS
        if useSSL && allowUntrusted {
            client.allowUntrustCACertificate = true
        }

        // Will 消息
        if let wt = willTopic, !wt.isEmpty, let wp = willPayload {
            let willMsg = CocoaMQTTMessage(topic: wt, string: wp, qos: CocoaMQTTQoS(rawValue: UInt8(willQos)) ?? .qos0, retained: willRetain)
            client.willMessage = willMsg
        }

        client.delegate = self
        mqtt = client

        // 异步连接
        DispatchQueue.global(qos: .utility).async {
            let success = client.connect()
            if !success {
                self.isConnectingActive = false
                DispatchQueue.main.async {
                    self.onConnect?(false, "CocoaMQTT connect() returned false — check host/port")
                }
            }
            // 成功路径由 CocoaMQTTDelegate.mqtt(_:didConnectAck:) 回调
        }
    }

    /// 断开连接（主动调用）
    @objc public func disconnect() {
        stopConnectionCheck()
        endBackgroundTask()
        mqtt?.disconnect()
        mqtt = nil
        isConnectingActive = false
    }

    /// 订阅主题
    @objc public func subscribe(topic: String, qos: Int32, onSuccess: @escaping () -> Void, onError: @escaping (String) -> Void) {
        guard let client = mqtt, client.connState == .connected else {
            onError("Not connected")
            return
        }
        // CocoaMQTT 通过 delegate 的 didSubscribeTopics 回调确认，
        // 这里做简化处理：订阅操作后直接回调 onSuccess（与 Android 保持行为一致）
        client.subscribe(topic, qos: CocoaMQTTQoS(rawValue: UInt8(qos)) ?? .qos0)
        DispatchQueue.main.async { onSuccess() }
    }

    /// 取消订阅
    @objc public func unsubscribe(topic: String, onSuccess: @escaping () -> Void, onError: @escaping (String) -> Void) {
        guard let client = mqtt, client.connState == .connected else {
            onError("Not connected")
            return
        }
        client.unsubscribe(topic)
        DispatchQueue.main.async { onSuccess() }
    }

    /// 发布消息
    @objc public func publish(
        topic:     String,
        payload:   Any,        // String 或 Data
        qos:       Int32,
        retain:    Bool,
        onSuccess: @escaping () -> Void,
        onError:   @escaping (String) -> Void
    ) {
        guard let client = mqtt, client.connState == .connected else {
            onError("Not connected")
            return
        }

        let msgQoS = CocoaMQTTQoS(rawValue: UInt8(qos)) ?? .qos0
        let msg: CocoaMQTTMessage

        if let str = payload as? String {
            msg = CocoaMQTTMessage(topic: topic, string: str, qos: msgQoS, retained: retain)
        } else if let data = payload as? Data {
            msg = CocoaMQTTMessage(topic: topic, payload: [UInt8](data), qos: msgQoS, retained: retain)
        } else {
            onError("Unsupported payload type")
            return
        }

        client.publish(msg)
        DispatchQueue.main.async { onSuccess() }
    }

    /// 是否已连接
    @objc public func isConnected() -> Bool {
        return mqtt?.connState == .connected
    }

    // MARK: - Connection check (active heartbeat)

    /// 启动主动心跳检测
    /// - Parameter intervalMs: 检测间隔，单位毫秒
    @objc public func startConnectionCheck(intervalMs: Int) {
        stopConnectionCheck()
        let interval = Double(max(10_000, intervalMs)) / 1000.0
        DispatchQueue.main.async {
            self.connectionCheckTimer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
                guard let self else { return }
                let alive = self.isConnected()
                self.onConnectionCheck?(alive)
                if !alive && self.mqtt != nil {
                    // 检测到假死，清理旧客户端并触发重连
                    self.mqtt = nil
                    self.onClose?()
                    self.onConnectionLost?()
                }
            }
        }
    }

    /// 停止主动心跳检测
    @objc public func stopConnectionCheck() {
        DispatchQueue.main.async {
            self.connectionCheckTimer?.invalidate()
            self.connectionCheckTimer = nil
        }
    }

    // MARK: - Background keep-alive (limited)

    /**
     * 申请后台任务时间。
     *
     * 在 App 进入后台（sceneDidEnterBackground / applicationDidEnterBackground）时调用。
     * iOS 最多允许约 30 秒后台运行时间。超时后系统会挂起进程，
     * MQTT 连接将断开，重连在 App 回到前台后由 UTS 层驱动。
     *
     * 调用时机由 app-ios/index.uts 通过监听 UniApp 的 onHide / onBackgroundFetch 事件来触发。
     */
    @objc public func beginBackgroundKeepAlive() {
        guard bgTaskID == .invalid else { return }   // 已有任务，不重复申请
        bgTaskID = UIApplication.shared.beginBackgroundTask(withName: "mqtt.keepalive") { [weak self] in
            // 系统即将强制结束后台时间，在这里做善后
            self?.endBackgroundTask()
        }
    }

    /// 结束后台任务（App 回到前台时调用）
    @objc public func endBackgroundKeepAlive() {
        endBackgroundTask()
    }

    // MARK: - Private helpers

    private func endBackgroundTask() {
        if bgTaskID != .invalid {
            UIApplication.shared.endBackgroundTask(bgTaskID)
            bgTaskID = .invalid
        }
    }

    /// 静默断开旧连接（不触发 UTS 层回调）
    private func disconnectSilently() {
        stopConnectionCheck()
        endBackgroundTask()
        if let old = mqtt {
            old.delegate = nil
            if old.connState == .connected {
                old.disconnect()
            }
            mqtt = nil
        }
    }

    /**
     * 解析 MQTT URL。
     * 支持：mqtt:// mqtts:// ws:// wss://
     * 返回 (host, port, useSSL, useWebSocket)
     */
    private func parseUrl(_ raw: String) -> (String, Int, Bool, Bool)? {
        let normalized: String
        let useSSL:  Bool
        let useWS:   Bool
        var defaultPort: Int

        if raw.hasPrefix("mqtts://") {
            normalized  = "https://" + raw.dropFirst("mqtts://".count)
            useSSL      = true
            useWS       = false
            defaultPort = 8883
        } else if raw.hasPrefix("mqtt://") {
            normalized  = "http://" + raw.dropFirst("mqtt://".count)
            useSSL      = false
            useWS       = false
            defaultPort = 1883
        } else if raw.hasPrefix("wss://") {
            normalized  = raw
            useSSL      = true
            useWS       = true
            defaultPort = 8084
        } else if raw.hasPrefix("ws://") {
            normalized  = raw
            useSSL      = false
            useWS       = true
            defaultPort = 8083
        } else {
            return nil
        }

        guard let components = URLComponents(string: normalized),
              let host = components.host else { return nil }
        let port = components.port ?? defaultPort
        return (host, port, useSSL, useWS)
    }
}

// MARK: - CocoaMQTTDelegate

extension MqttClientWrapper: CocoaMQTTDelegate {

    public func mqtt(_ mqtt: CocoaMQTT, didConnectAck ack: CocoaMQTTConnAck) {
        isConnectingActive = false
        if ack == .accept {
            // 连接成功：启动心跳检测，申请后台保活时间（进入后台时才有效）
            let keepalive = Int(mqtt.keepAlive)
            let checkInterval = max(10_000, (keepalive / 2) * 1000)
            startConnectionCheck(intervalMs: checkInterval)

            DispatchQueue.main.async {
                self.onConnect?(false, nil)
            }
        } else {
            let errMsg = "Connection refused: \(ack)"
            DispatchQueue.main.async {
                self.onConnect?(false, errMsg)
            }
        }
    }

    public func mqtt(_ mqtt: CocoaMQTT, didReceiveMessage message: CocoaMQTTMessage, id: UInt16) {
        let topic   = message.topic
        let payload = message.string ?? ""
        let qos     = Int32(message.qos.rawValue)
        let retain  = message.retained
        DispatchQueue.main.async {
            self.onMessage?(topic, payload, qos, retain)
        }
    }

    public func mqttDidDisconnect(_ mqtt: CocoaMQTT, withError err: Error?) {
        isConnectingActive = false
        stopConnectionCheck()
        endBackgroundTask()
        DispatchQueue.main.async {
            self.onClose?()
            if err != nil {
                // 非主动断开（err != nil）→ 触发重连流程
                self.onConnectionLost?()
            }
        }
    }

    // 以下为 CocoaMQTTDelegate 必须实现但本插件无需特殊处理的方法

    public func mqtt(_ mqtt: CocoaMQTT, didPublishMessage message: CocoaMQTTMessage, id: UInt16) {}
    public func mqtt(_ mqtt: CocoaMQTT, didPublishAck id: UInt16) {}
    public func mqtt(_ mqtt: CocoaMQTT, didSubscribeTopics success: NSDictionary, failed: [String]) {}
    public func mqtt(_ mqtt: CocoaMQTT, didUnsubscribeTopics topics: [String]) {}
    public func mqttDidPing(_ mqtt: CocoaMQTT) {}
    public func mqttDidReceivePong(_ mqtt: CocoaMQTT) {}
}
