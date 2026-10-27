package uni.changbi.mqtt

import android.content.Context
import android.content.SharedPreferences
import android.os.Handler
import android.os.Looper
import android.os.PowerManager
import android.util.Log
import org.eclipse.paho.client.mqttv3.*
import org.eclipse.paho.client.mqttv3.persist.MemoryPersistence
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import javax.net.ssl.SSLContext
import javax.net.ssl.TrustManager
import javax.net.ssl.X509TrustManager
import java.security.cert.X509Certificate

/**
 * MQTT 辅助类 - 纯 Eclipse Paho Java Client + WorkManager 后台心跳
 *
 * 变更说明：
 *   1. 改为单例（companion object getInstance），供 MqttHeartbeatWorker 访问同一实例。
 *   2. connect() 成功后自动调用 MqttHeartbeatWorker.schedule() 注册后台心跳。
 *   3. disconnect() 时调用 MqttHeartbeatWorker.cancel() 取消心跳。
 */
class MqttHelper private constructor(private val context: Context) {

    @Volatile
    private var mqttClient: MqttClient? = null

    @Volatile
    private var isConnectingActive: Boolean = false

    private val executor: ScheduledExecutorService = Executors.newScheduledThreadPool(2)

    // WakeLock：阻止 CPU 在息屏后休眠，保证 keepalive 包能按时发出
    // PARTIAL_WAKE_LOCK 只保持 CPU 运行，不会点亮屏幕，对电量影响极小
    private val wakeLock: PowerManager.WakeLock by lazy {
        val pm = context.getSystemService(Context.POWER_SERVICE) as PowerManager
        pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "changbi:MqttWakeLock").apply {
            setReferenceCounted(false) // 非引用计数模式，acquire/release 成对调用即可
        }
    }

    // 回调
    @Volatile
    private var connectCallback: ((Boolean, String?) -> Unit)? = null
    // 消息回调：Kotlin 层完成 ByteArray→UTF-8 String 转换后再回调 UTS 侧，
    // 避免 UTS 编译器将 new ArrayBuffer() 编译为 Java byte[]，导致 ClassCastException 崩溃
    @Volatile
    private var messageCallback: ((String, String, Int, Boolean) -> Unit)? = null
    @Volatile
    private var closeCallback: (() -> Unit)? = null
    @Volatile
    private var connectionLostCallback: (() -> Unit)? = null
    // 主动心跳检测：连接假死时的通知回调
    @Volatile
    private var connectionCheckCallback: ((Boolean) -> Unit)? = null
    private var connectionCheckFuture: java.util.concurrent.ScheduledFuture<*>? = null

    // 保存最近一次连接参数，供 Worker 重连时使用
    private var lastUrl: String = ""
    private var lastClientId: String = ""
    private var lastUsername: String? = null
    private var lastPassword: String? = null
    private var lastClean: Boolean = true
    private var lastTimeout: Int = 30000
    private var lastKeepalive: Int = 60
    private var lastAllowUntrusted: Boolean = false

    private fun runOnMainThread(action: () -> Unit) {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            action()
        } else {
            Handler(Looper.getMainLooper()).post(action)
        }
    }

    companion object {
        private const val TAG = "MqttHelper"

        @Volatile
        private var instance: MqttHelper? = null

        /**
         * 获取单例。
         * MqttHeartbeatWorker 通过此方法拿到与 index.uts 相同的实例，
         * 避免创建第二个独立的 MQTT 连接。
         */
        fun getInstance(context: Context): MqttHelper {
            return instance ?: synchronized(this) {
                instance ?: MqttHelper(context.applicationContext).also { instance = it }
            }
        }
    }

    // 回调注册

    fun setConnectCallback(callback: (Boolean, String?) -> Unit) {
        this.connectCallback = callback
    }

    fun setMessageCallback(callback: (String, String, Int, Boolean) -> Unit) {
        this.messageCallback = callback
    }

    fun setCloseCallback(callback: () -> Unit) {
        this.closeCallback = callback
    }

    fun setConnectionLostCallback(callback: () -> Unit) {
        this.connectionLostCallback = callback
    }

    /**
     * 设置主动心跳检测结果回调
     * isAlive=true 表示连接正常，false 表示检测到假死
     */
    fun setConnectionCheckCallback(callback: (Boolean) -> Unit) {
        this.connectionCheckCallback = callback
    }

    fun startConnectionCheck(intervalMs: Long) {
        stopConnectionCheck()
        connectionCheckFuture = (executor as java.util.concurrent.ScheduledExecutorService)
            .scheduleWithFixedDelay({
                // 如果当前正在连接初始化中，跳过本次检测，避免因连接尚未完成而误判为断线
                if (isConnectingActive) {
                    Log.d(TAG, "Connection check: actively connecting, skip check")
                    return@scheduleWithFixedDelay
                }
                val alive = mqttClient?.isConnected ?: false
                runOnMainThread {
                    connectionCheckCallback?.invoke(alive)
                }
                if (!alive && mqttClient != null) {
                    // Paho 认为已断线但还没触发回调，手动补触发
                    Log.w(TAG, "Connection check: detected dead connection, triggering lost callback")
                    runOnMainThread {
                        closeCallback?.invoke()
                        connectionLostCallback?.invoke()
                    }
                    // 清掉引用避免重复触发
                    synchronized(this) {
                        mqttClient = null
                    }
                }
            }, intervalMs, intervalMs, java.util.concurrent.TimeUnit.MILLISECONDS)
        Log.d(TAG, "Connection check scheduled every ${intervalMs}ms")
    }

    /**
     * 停止主动心跳检测
     */
    fun stopConnectionCheck() {
        connectionCheckFuture?.cancel(false)
        connectionCheckFuture = null
    }

    // 连接
  
    fun connect(
        url: String,
        clientId: String,
        username: String?,
        password: String?,
        clean: Boolean,
        timeout: Int,
        keepalive: Int,
        allowUntrusted: Boolean = false,
        willTopic: String?,
        willPayload: String?,
        willQos: Int,
        willRetain: Boolean,
        onSuccess: () -> Unit,
        onError: (String) -> Unit
    ) {
        if (isConnectingActive) {
            Log.w(TAG, "Connect requested while connection is already in progress. Ignoring.")
            runOnMainThread {
                onError("Connection already in progress")
            }
            return
        }
        isConnectingActive = true
        executor.execute {
            try {
                val serverUri = normalizeServerUri(url)

                synchronized(this) {
                    safeDisposePreviousClient()
                    val persistence = MemoryPersistence()
                    mqttClient = MqttClient(serverUri, clientId, persistence)
                }

                val options = MqttConnectOptions().apply {
                    isCleanSession = clean
                    connectionTimeout = maxOf(1, timeout / 1000)
                    keepAliveInterval = keepalive
                    isAutomaticReconnect = false

                    if (!username.isNullOrEmpty()) userName = username
                    if (!password.isNullOrEmpty()) setPassword(password.toCharArray())

                    // Will 消息：只有 willTopic 非空时才设置
                    if (!willTopic.isNullOrEmpty() && willPayload != null) {
                        setWill(willTopic, willPayload.toByteArray(Charsets.UTF_8), willQos, willRetain)
                    }

                    if (serverUri.startsWith("ssl://") || serverUri.startsWith("wss://")) {
                        socketFactory = if (allowUntrusted) {
                            createTrustAllSocketFactory()
                        } else {
                            SSLContext.getInstance("TLS").apply { init(null, null, null) }.socketFactory
                        }
                    }
                }

                mqttClient?.setCallback(object : MqttCallbackExtended {
                    override fun connectComplete(reconnect: Boolean, serverURI: String?) {}

                    override fun connectionLost(cause: Throwable?) {
                        Log.w(TAG, "Connection lost: ${cause?.message}")
                        runOnMainThread {
                            closeCallback?.invoke()
                            connectionLostCallback?.invoke()
                        }
                    }

                    override fun messageArrived(topic: String?, message: MqttMessage?) {
                        if (topic != null && message != null) {
                            // 在 Kotlin 层将 ByteArray 转换为 UTF-8 字符串，
                            // 不再将原始 ByteArray 传递给 UTS 侧，避免 ArrayBuffer 类型转换崩溃
                            val payloadStr = try {
                                String(message.payload, Charsets.UTF_8)
                            } catch (e: Exception) {
                                Log.w(TAG, "Failed to decode payload as UTF-8, using empty string: ${e.message}")
                                ""
                            }
                            val qos = message.qos
                            val retained = message.isRetained
                            runOnMainThread {
                                messageCallback?.invoke(topic, payloadStr, qos, retained)
                            }
                        }
                    }

                    override fun deliveryComplete(token: IMqttDeliveryToken?) {}
                })

                mqttClient?.connect(options)

                // 连接成功：保存参数 + 启动 WorkManager 心跳
                lastUrl            = url
                lastClientId       = clientId
                lastUsername       = username
                lastPassword       = password
                lastClean          = clean
                lastTimeout        = timeout
                lastKeepalive      = keepalive
                lastAllowUntrusted = allowUntrusted

                saveToPreferences(url, clientId, username, password, clean, timeout, keepalive, allowUntrusted)

                // 自动恢复本地保存的订阅
                restoreSubscriptions()

                MqttHeartbeatWorker.schedule(context)

                // 启动前台服务，防止进程被系统回收
                MqttForegroundService.start(context)

                // 申请 WakeLock，阻止 CPU 休眠，保证 keepalive 心跳包按时发出
                // 息屏后没有 WakeLock，CPU 会休眠，socket I/O 冻结，broker 会认为客户端超时断线
                if (!wakeLock.isHeld) {
                    wakeLock.acquire()
                    Log.d(TAG, "WakeLock acquired")
                }

                // 启动主动心跳检测，间隔 = keepalive/2（最少 10s）
                val checkInterval = maxOf(10L, (keepalive / 2).toLong()) * 1000L
                startConnectionCheck(checkInterval)

                Log.d(TAG, "Connected to $serverUri")
                isConnectingActive = false
                runOnMainThread {
                    connectCallback?.invoke(false, null)
                    onSuccess()
                }

            } catch (e: Exception) {
                isConnectingActive = false
                Log.e(TAG, "Connect failed: ${e.message}")
                val errMsg = buildErrorMessage(e, url)
                runOnMainThread {
                    onError(errMsg)
                }
            }
        }
    }

    // 断开连接

    fun disconnect(onSuccess: () -> Unit, onError: (String) -> Unit) {
        isConnectingActive = false
        executor.execute {
            try {
                // 停止主动心跳检测 + 取消后台心跳 + 停止前台服务
                stopConnectionCheck()
                MqttHeartbeatWorker.cancel(context)
                MqttForegroundService.stop(context)

                // 释放 WakeLock
                if (wakeLock.isHeld) {
                    wakeLock.release()
                    Log.d(TAG, "WakeLock released")
                }

                // 标记连接为已禁用（显式主动断开），防止后台心跳 Worker 误触发重连
                try {
                    context.getSharedPreferences("mqtt_helper_prefs", Context.MODE_PRIVATE)
                        .edit()
                        .putBoolean("enabled", false)
                        .apply()
                    Log.d(TAG, "MQTT explicitly disabled in SharedPreferences")
                } catch (pe: Exception) {
                    Log.e(TAG, "Failed to update enabled flag in SharedPreferences: ${pe.message}")
                }

                // 清理 SharedPreferences 中的订阅
                clearSubscriptionsFromPreferences()

                val client = synchronized(this) {
                    val c = mqttClient
                    mqttClient = null
                    c
                }
                if (client != null) {
                    try {
                        // 无论 isConnected() 返回什么都尝试断开，
                        // 避免中间态（如连接丢失回调尚未到达）导致 disconnect 被跳过，
                        // broker 侧却仍认为客户端在线。
                        if (client.isConnected) {
                            client.disconnect(5000)
                        } else {
                            // 不在 connected 状态时用 disconnectForcibly 确保资源释放
                            client.disconnectForcibly(500, 500)
                        }
                    } catch (_: Exception) {}
                    try { client.close(true) } catch (_: Exception) {}
                }
                // 注意：这里不调用 closeCallback。
                // closeCallback 专用于"连接意外中断"（connectionLost），
                // 主动 disconnect 的状态更新由 index.uts 的 onSuccess 回调负责。
                Log.d(TAG, "Disconnected")
                runOnMainThread {
                    onSuccess()
                }
            } catch (e: Exception) {
                synchronized(this) {
                    mqttClient = null
                }
                val errMsg = getErrorMessage(e, "Disconnect failed")
                runOnMainThread {
                    onError(errMsg)
                }
            }
        }
    }

    // 订阅 / 取消订阅 / 发布

    fun subscribe(topic: String, qos: Int, onSuccess: () -> Unit, onError: (String) -> Unit) {
        executor.execute {
            try {
                requireConnected()
                mqttClient?.subscribe(topic, qos)
                saveSubscriptionToPreferences(topic, qos)
                runOnMainThread {
                    onSuccess()
                }
            } catch (e: Exception) {
                val errMsg = getErrorMessage(e, "Subscribe failed")
                runOnMainThread {
                    onError(errMsg)
                }
            }
        }
    }

    fun unsubscribe(topic: String, onSuccess: () -> Unit, onError: (String) -> Unit) {
        executor.execute {
            try {
                requireConnected()
                mqttClient?.unsubscribe(topic)
                removeSubscriptionFromPreferences(topic)
                runOnMainThread {
                    onSuccess()
                }
            } catch (e: Exception) {
                val errMsg = getErrorMessage(e, "Unsubscribe failed")
                runOnMainThread {
                    onError(errMsg)
                }
            }
        }
    }

    fun publish(
        topic: String,
        payload: Any,
        qos: Int,
        retain: Boolean,
        onSuccess: () -> Unit,
        onError: (String) -> Unit
    ) {
        executor.execute {
            try {
                requireConnected()
                val bytes = when (payload) {
                    is String -> payload.toByteArray(Charsets.UTF_8)
                    is ByteArray -> payload
                    is java.nio.ByteBuffer -> {
                        val arr = ByteArray(payload.remaining())
                        payload.get(arr)
                        arr
                    }
                    else -> payload.toString().toByteArray(Charsets.UTF_8)
                }
                val message = MqttMessage(bytes).apply {
                    this.qos = qos
                    this.isRetained = retain
                }
                mqttClient?.publish(topic, message)
                runOnMainThread {
                    onSuccess()
                }
            } catch (e: Exception) {
                val errMsg = getErrorMessage(e, "Publish failed")
                runOnMainThread {
                    onError(errMsg)
                }
            }
        }
    }

    // 状态查询 / 清理

    fun isConnected(): Boolean = mqttClient?.isConnected ?: false

    fun cleanup() {
        try {
            stopConnectionCheck()
            MqttHeartbeatWorker.cancel(context)
            MqttForegroundService.stop(context)
            if (wakeLock.isHeld) {
                wakeLock.release()
                Log.d(TAG, "WakeLock released in cleanup")
            }
            safeDisposePreviousClient()
        } catch (_: Exception) {}
    }

    // 私有辅助方法

    private fun saveToPreferences(
        url: String,
        clientId: String,
        username: String?,
        password: String?,
        clean: Boolean,
        timeout: Int,
        keepalive: Int,
        allowUntrusted: Boolean
    ) {
        try {
            val prefs = context.getSharedPreferences("mqtt_helper_prefs", Context.MODE_PRIVATE)
            prefs.edit().apply {
                putString("url", url)
                putString("clientId", clientId)
                putString("username", username)
                putString("password", password)
                putBoolean("clean", clean)
                putInt("timeout", timeout)
                putInt("keepalive", keepalive)
                putBoolean("allowUntrusted", allowUntrusted)
                putBoolean("enabled", true) // 显式允许重连
                apply()
            }
            Log.d(TAG, "Connection parameters saved to SharedPreferences")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save connection parameters to SharedPreferences: ${e.message}")
        }
    }

    private fun saveSubscriptionToPreferences(topic: String, qos: Int) {
        try {
            context.getSharedPreferences("mqtt_subscriptions", Context.MODE_PRIVATE)
                .edit()
                .putInt(topic, qos)
                .apply()
            Log.d(TAG, "Saved subscription to SharedPreferences: $topic (QoS $qos)")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save subscription to SharedPreferences: ${e.message}")
        }
    }

    private fun removeSubscriptionFromPreferences(topic: String) {
        try {
            context.getSharedPreferences("mqtt_subscriptions", Context.MODE_PRIVATE)
                .edit()
                .remove(topic)
                .apply()
            Log.d(TAG, "Removed subscription from SharedPreferences: $topic")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to remove subscription: ${e.message}")
        }
    }

    private fun clearSubscriptionsFromPreferences() {
        try {
            context.getSharedPreferences("mqtt_subscriptions", Context.MODE_PRIVATE)
                .edit()
                .clear()
                .apply()
            Log.d(TAG, "Cleared all subscriptions from SharedPreferences")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to clear subscriptions: ${e.message}")
        }
    }

    private fun restoreSubscriptions() {
        try {
            val prefs = context.getSharedPreferences("mqtt_subscriptions", Context.MODE_PRIVATE)
            val all = prefs.all
            if (!all.isEmpty()) {
                Log.d(TAG, "Restoring ${all.size} subscriptions from SharedPreferences...")
                for ((topic, qosObj) in all) {
                    val qos = (qosObj as? Int) ?: 0
                    try {
                        mqttClient?.subscribe(topic, qos)
                        Log.d(TAG, "Restored subscription: $topic (QoS $qos)")
                    } catch (se: Exception) {
                        Log.e(TAG, "Failed to restore subscription for $topic: ${se.message}")
                    }
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to restore subscriptions: ${e.message}")
        }
    }

    private fun requireConnected() {
        if (mqttClient?.isConnected != true) {
            throw IllegalStateException("MQTT client is not connected")
        }
    }

    private fun safeDisposePreviousClient() {
        try {
            // 停止之前的心跳 Worker，避免旧 Worker 在新连接建立后仍用旧参数重连
            MqttHeartbeatWorker.cancel(context)
            val old = synchronized(this) {
                val c = mqttClient
                mqttClient = null
                c
            } ?: return
            if (old.isConnected) old.disconnectForcibly(500, 500)
            old.close(true)
        } catch (_: Exception) {}
    }

    private fun normalizeServerUri(url: String): String = when {
        url.startsWith("mqtt://")  -> url.replaceFirst("mqtt://", "tcp://")
        url.startsWith("mqtts://") -> url.replaceFirst("mqtts://", "ssl://")
        else -> url
    }

    private fun createTrustAllSocketFactory(): javax.net.ssl.SSLSocketFactory {
        val trustAllCerts = arrayOf<TrustManager>(object : X509TrustManager {
            override fun checkClientTrusted(chain: Array<X509Certificate>, authType: String) {}
            override fun checkServerTrusted(chain: Array<X509Certificate>, authType: String) {}
            override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
        })
        val sc = SSLContext.getInstance("TLS")
        sc.init(null, trustAllCerts, java.security.SecureRandom())
        return sc.socketFactory
    }

    private fun buildErrorMessage(e: Exception, originalUrl: String): String {
        val sb = StringBuilder()
        when (e) {
            is MqttException -> {
                val code = e.reasonCode
                sb.append("MQTT Error (code: $code)")
                when (code.toInt()) {
                    0     -> sb.append(" - Connection refused or network error")
                    32100 -> sb.append(" - Client is not connected")
                    32101 -> sb.append(" - Client is already connected")
                    32102 -> sb.append(" - Client is disconnecting")
                    32103 -> sb.append(" - Server unavailable")
                    32104 -> sb.append(" - Bad username or password")
                    32105 -> sb.append(" - Not authorized")
                    32109 -> sb.append(" - Connection lost")
                }
                if (originalUrl.startsWith("wss://") && code.toInt() == 0) {
                    sb.append("\n💡 WSS 连接失败常见原因：SSL 证书问题、服务器端口未开放、网络防火墙拦截")
                }
                sb.append("\n📋 详情: ${e.message}")
                appendCauseChain(sb, e.cause)
            }
            else -> {
                sb.append(e.message ?: "Unknown error")
                appendCauseChain(sb, e.cause)
            }
        }
        return sb.toString()
    }

    private fun appendCauseChain(sb: StringBuilder, cause: Throwable?, maxDepth: Int = 3) {
        var current = cause
        var depth = 1
        while (current != null && depth <= maxDepth) {
            sb.append("\n  └─ Cause $depth: ${current.message ?: current.javaClass.simpleName}")
            current = current.cause
            depth++
        }
    }

    private fun getErrorMessage(e: Throwable?, defaultMessage: String): String {
        if (e == null) return defaultMessage
        val sb = StringBuilder()
        sb.append(e.message ?: defaultMessage)
        appendCauseChain(sb, e.cause)
        return sb.toString()
    }
}