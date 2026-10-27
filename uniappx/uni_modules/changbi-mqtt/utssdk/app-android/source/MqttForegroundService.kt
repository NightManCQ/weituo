package uni.changbi.mqtt

import android.app.*
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat

/**
 * MQTT 前台服务
 *
 * 作用：
 *   Android 对后台进程会积极回收内存，普通后台服务随时可能被杀。
 *   前台服务会在通知栏显示一条持久通知，系统将其视为"用户感知进程"，
 *   优先级与前台 Activity 相当，极少被回收。
 *
 *   运行期间 MQTT 连接由 MqttHelper 维持，断线由 connectionLostCallback
 *   触发重连逻辑（已在 index.uts 的 ConnectionManager 中实现）。
 *   WorkManager 心跳作为双重兜底继续保留。
 *
 * 生命周期：
 *   - 调用 MqttForegroundService.start(context) 启动
 *   - 调用 MqttForegroundService.stop(context)  停止
 *   - 服务被系统意外杀死后，START_STICKY 会让系统自动重启它
 *
 * Android 版本适配：
 *   - Android 8+  (API 26)：必须通过 startForegroundService() 启动，且在 5 秒内调用 startForeground()
 *   - Android 12+ (API 31)：新增 foregroundServiceType 限制，使用 "dataSync" 类型
 *   - Android 14+ (API 34)：dataSync 类型需要在 manifest 中声明，已在 AndroidManifest.xml 处理
 */
class MqttForegroundService : Service() {

    companion object {
        private const val TAG = "MqttForegroundService"
        private const val NOTIFICATION_ID = 0x4D515454  // "MQTT" ASCII
        private const val CHANNEL_ID = "mqtt_foreground_channel"
        private const val CHANNEL_NAME = "MQTT 连接保持"

        // Intent Action
        private const val ACTION_START = "uni.changbi.mqtt.ACTION_START"
        private const val ACTION_STOP  = "uni.changbi.mqtt.ACTION_STOP"

        // 通知文字（可按需修改）
        private const val NOTIFICATION_TITLE   = "MQTT 连接中"
        private const val NOTIFICATION_CONTENT = "正在保持消息连接，请勿清除此通知"

        /**
         * 启动前台服务
         * 必须在主线程或有效的 Context 中调用
         */
        fun start(context: Context) {
            try {
                val intent = Intent(context, MqttForegroundService::class.java).apply {
                    action = ACTION_START
                }
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    context.startForegroundService(intent)
                } else {
                    context.startService(intent)
                }
                Log.d(TAG, "Service start requested")
            } catch (e: Exception) {
                Log.e(TAG, "Failed to start foreground service: ${e.message}")
            }
        }

        /**
         * 停止前台服务（主动断开 MQTT 时调用）
         */
        fun stop(context: Context) {
            try {
                val intent = Intent(context, MqttForegroundService::class.java)
                context.stopService(intent)
                Log.d(TAG, "Service stop requested via stopService")
            } catch (e: Exception) {
                Log.e(TAG, "Failed to stop foreground service: ${e.message}")
            }
        }
    }

    // Service 生命周期
   
    override fun onCreate() {
        super.onCreate()
        Log.d(TAG, "Service created")
        createNotificationChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> {
                Log.d(TAG, "Stopping foreground service")
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
                    stopForeground(STOP_FOREGROUND_REMOVE)
                } else {
                    @Suppress("DEPRECATION")
                    stopForeground(true)
                }
                stopSelf()
            }
            else -> {
                // ACTION_START 或系统以 START_STICKY 重启（intent 为 null）都进入此分支
                Log.d(TAG, "Starting foreground service (intent action: ${intent?.action ?: "null/STICKY_RESTART"})")
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    startForeground(
                        NOTIFICATION_ID,
                        buildNotification(),
                        ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
                    )
                } else {
                    startForeground(NOTIFICATION_ID, buildNotification())
                }
                // intent==null 说明服务是被系统 START_STICKY 重启的。
                // 此时进程刚恢复，UTS 层全局状态尚未初始化，不应在此处尝试重连。
                // 等待 App 回到前台后由 UTS 层自行 connect()。
                // 注意：以前在此触发 triggerImmediateHeartbeat() 会导致 Kotlin 层建立
                // 一个 UTS 层无法感知的"无主连接"——即用户看到的"每15分钟连上即断"现象。
                if (intent == null) {
                    Log.d(TAG, "Service restarted by system, waiting for App to reconnect via UTS layer.")
                }
            }
        }

        // START_STICKY：服务被系统杀死后，系统会自动重启，intent 传 null
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null  // 不支持绑定模式

    override fun onDestroy() {
        super.onDestroy()
        Log.d(TAG, "Service destroyed")
    }

    // 通知

    /**
     * Android 8+ 必须先创建 NotificationChannel 才能发通知
     */
    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                CHANNEL_NAME,
                NotificationManager.IMPORTANCE_LOW  // LOW = 无声音/震动，不打扰用户
            ).apply {
                description = "用于保持 MQTT 消息连接的后台服务通知"
                setShowBadge(false)
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(): Notification {
        // 点击通知时跳转到 App 主界面
        val pendingIntentFlags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        } else {
            PendingIntent.FLAG_UPDATE_CURRENT
        }

        val launchIntent = packageManager.getLaunchIntentForPackage(packageName)
        val pendingIntent = PendingIntent.getActivity(
            this, 0, launchIntent ?: Intent(), pendingIntentFlags
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(NOTIFICATION_TITLE)
            .setContentText(NOTIFICATION_CONTENT)
            // 使用系统内置小图标，避免自定义图标资源找不到导致崩溃
            .setSmallIcon(android.R.drawable.stat_notify_sync)
            .setContentIntent(pendingIntent)
            .setOngoing(true)          // 用户不能手动划掉
            .setShowWhen(false)        // 不显示时间戳
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }
}