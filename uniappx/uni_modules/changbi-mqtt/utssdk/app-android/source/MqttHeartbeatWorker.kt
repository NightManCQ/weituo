package uni.changbi.mqtt

import android.content.Context
import android.util.Log
import androidx.work.*
import java.util.concurrent.TimeUnit

/**
 * MQTT 后台心跳 Worker
 *
 * 职责：
 *   - 由 WorkManager 定期唤醒（最小间隔 15 分钟，系统可批处理）
 *   - 检查 MqttHelper 单例的连接状态
 *   - 若已断开且保存了连接参数，则尝试重新连接
 *   - 连接参数通过 WorkManager Data 持久化，进程被杀后恢复也能读到
 *
 * 注意事项：
 *   - Worker 运行在后台线程，不可操作 UI
 *   - doWork() 必须在有限时间内返回（WorkManager 默认超时 10 分钟）
 *   - 使用同步阻塞的 MqttClient.connect()，配合超时保证不卡死
 */
class MqttHeartbeatWorker(
    private val appContext: Context,
    workerParams: WorkerParameters
) : Worker(appContext, workerParams) {

    companion object {
        private const val TAG = "MqttHeartbeat"

        // WorkManager 任务唯一名称（同名任务只会有一个在队列中）
        const val WORK_NAME = "mqtt_heartbeat_work"

        /**
         * 启动/更新心跳调度
         *
         * Worker 职责仅限于"检活并记录日志"，不再传递连接参数，不再负责重连。
         * 重连由 MqttHelper.connectionLostCallback → UTS 层 ConnectionManager 负责。
         *
         * repeatInterval 设为 15 分钟（WorkManager 允许的最小值）。
         */
        fun schedule(context: Context) {
            val constraints = Constraints.Builder()
                .setRequiredNetworkType(NetworkType.CONNECTED)
                .build()

            val request = PeriodicWorkRequestBuilder<MqttHeartbeatWorker>(
                repeatInterval = 15,
                repeatIntervalTimeUnit = TimeUnit.MINUTES
            )
                .setConstraints(constraints)
                .setInitialDelay(15, TimeUnit.MINUTES)
                .build()

            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                WORK_NAME,
                ExistingPeriodicWorkPolicy.UPDATE,
                request
            )

            Log.d(TAG, "Heartbeat worker scheduled")
        }

        /**
         * 取消心跳调度（断开连接时调用）
         */
        fun cancel(context: Context) {
            WorkManager.getInstance(context).cancelUniqueWork(WORK_NAME)
            Log.d(TAG, "Heartbeat worker cancelled")
        }
    }

    override fun doWork(): Result {
        Log.d(TAG, "Heartbeat check started")

        val prefs = appContext.getSharedPreferences("mqtt_helper_prefs", Context.MODE_PRIVATE)

        // 用户主动 disconnect() 后 enabled=false，无需任何操作
        val isEnabled = prefs.getBoolean("enabled", false)
        if (!isEnabled) {
            Log.d(TAG, "MQTT explicitly disabled by user, skipping heartbeat check.")
            return Result.success()
        }

        // 获取 MqttHelper 单例，仅做连接状态检查
        // ⚠️ Worker 不负责重连。
        //
        // 原因：进程被系统杀死后，UTS 层的全局状态（mqttHelper 引用、
        // connectCallback / messageCallback / connectionLostCallback 等所有回调）
        // 全部丢失。此时即使 Kotlin 层 MqttHelper 能重新建立 TCP 连接，
        // UTS 层也无法感知 connect / message / connectionLost 事件，
        // 连接将成为"无主连接"并在数秒内因缺少上层维护而断开——
        // 即用户看到的"每15分钟连上就断"现象。
        //
        // 正确的恢复路径：
        //   ForegroundService START_STICKY 拉起进程
        //   → App 回到前台后 UTS 层重新执行 connect()
        //   → UTS 层重新注册所有回调，连接才真正可用
        //
        // Worker 的职责仅限于：当 App 进程存活且 UTS 层认为应该连接时，
        // 检测到 Paho 层意外断线并记录日志，真正的重连由
        // MqttHelper 的 connectionLostCallback → ConnectionManager 负责。
        val helper = MqttHelper.getInstance(appContext)

        return if (helper.isConnected()) {
            Log.d(TAG, "Heartbeat OK: connection alive.")
            Result.success()
        } else {
            // 连接不存在：可能是进程刚被拉起（UTS 层尚未 connect），
            // 也可能是 connectionLostCallback 尚未触发重连。
            // 两种情况下 Worker 都不介入，只记录日志，等待 UTS 层自行处理。
            Log.w(TAG, "Heartbeat: connection not alive. Waiting for UTS layer to reconnect.")
            Result.success()
        }
    }
}