package uni.changbi.mqtt

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/**
 * 开机自启广播接收器
 *
 * 作用：
 *   设备重启或 App 更新后，系统会发送 BOOT_COMPLETED / MY_PACKAGE_REPLACED 广播。
 *   收到广播后检查 SharedPreferences 里是否有保存的连接参数，
 *   如果有且用户未主动 disconnect（enabled=true），则重启前台服务。
 *   MqttForegroundService 收到 intent=null 的 START_STICKY 重启时会触发
 *   MqttHeartbeatWorker 一次性任务来完成实际重连。
 */
class MqttBootReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "MqttBootReceiver"
    }

    override fun onReceive(context: Context, intent: Intent?) {
        val action = intent?.action ?: return

        if (action != Intent.ACTION_BOOT_COMPLETED &&
            action != Intent.ACTION_MY_PACKAGE_REPLACED) {
            return
        }

        Log.d(TAG, "Received $action")

        // 检查用户是否启用了 MQTT 连接（主动 disconnect 会把 enabled 设为 false）
        val prefs = context.getSharedPreferences("mqtt_helper_prefs", Context.MODE_PRIVATE)
        val isEnabled = prefs.getBoolean("enabled", false)
        val url = prefs.getString("url", null)

        if (!isEnabled || url.isNullOrEmpty()) {
            Log.d(TAG, "MQTT not enabled or no saved params, skipping auto-start")
            return
        }

        Log.d(TAG, "Auto-starting MQTT foreground service after boot")
        MqttForegroundService.start(context)
    }
}
