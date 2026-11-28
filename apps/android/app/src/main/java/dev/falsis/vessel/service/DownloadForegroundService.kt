package dev.falsis.vessel.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import dev.falsis.vessel.MainActivity
import dev.falsis.vessel.data.DownloadProgressSnapshot
import dev.falsis.vessel.data.StoredDownload
import dev.falsis.vessel.data.VesselRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * Keeps offline media downloads running reliably when Vessel moves to the background on Android.
 *
 * Observes active/queued items in [VesselRepository.downloadProgress] and updates a single
 * foreground notification with progress, speed estimate, and a Pause All quick action.
 */
class DownloadForegroundService : Service() {
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var observerJob: Job? = null
    private var lastBytesSample: Long = 0L
    private var lastSampleAtMs: Long = 0L
    private var lastSpeedBps: Long = 0L

    override fun onCreate() {
        super.onCreate()
        ensureChannel()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_PAUSE_ALL -> {
                serviceScope.launch {
                    val repo = VesselRepository.get(applicationContext)
                    val active = repo.listDownloads()
                        .filter { it.status == "downloading" || it.status == "queued" }
                    for (dl in active) {
                        runCatching { repo.pauseDownload(dl.id) }
                    }
                    stopSelfSafely()
                }
                return START_NOT_STICKY
            }
            ACTION_STOP -> {
                stopSelfSafely()
                return START_NOT_STICKY
            }
        }

        startForegroundCompat(buildNotification(title = "Vessel Downloads", text = "Preparing offline transfer…", percent = null))
        startObserving()
        return START_STICKY
    }

    private fun startObserving() {
        if (observerJob != null) return
        val repo = VesselRepository.get(applicationContext)
        observerJob = serviceScope.launch {
            while (true) {
                val downloads = runCatching { repo.listDownloads() }.getOrDefault(emptyList())
                val snapshots = repo.downloadProgress.value
                val active = downloads.filter { dl ->
                    val status = snapshots[dl.id]?.status ?: dl.status
                    status == "downloading" || status == "queued"
                }

                if (active.isEmpty()) {
                    stopSelfSafely()
                    return@launch
                }

                val notification = renderProgressNotification(active, snapshots)
                val mgr = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
                mgr.notify(NOTIFICATION_ID, notification)
                delay(1000L)
            }
        }
    }

    private fun renderProgressNotification(
        active: List<StoredDownload>,
        snapshots: Map<String, DownloadProgressSnapshot>,
    ): Notification {
        val current = active.firstOrNull { (snapshots[it.id]?.status ?: it.status) == "downloading" }
            ?: active.first()
        val snap = snapshots[current.id]
        val bytesDownloaded = snap?.bytesDownloaded ?: current.bytesDownloaded
        val totalBytes = snap?.totalBytes ?: current.totalBytes
        val segmentsDone = snap?.segmentsDone ?: current.segmentsDone
        val segmentsTotal = snap?.segmentsTotal ?: current.segmentsTotal

        val totalDownloadedAcrossActive = active.sumOf { snapshots[it.id]?.bytesDownloaded ?: it.bytesDownloaded }
        val nowMs = System.currentTimeMillis()
        if (lastSampleAtMs > 0L && nowMs > lastSampleAtMs) {
            val dtMs = nowMs - lastSampleAtMs
            val dBytes = (totalDownloadedAcrossActive - lastBytesSample).coerceAtLeast(0L)
            lastSpeedBps = (dBytes * 1000L) / dtMs
        }
        lastSampleAtMs = nowMs
        lastBytesSample = totalDownloadedAcrossActive

        val percent = when {
            segmentsTotal > 0 -> ((segmentsDone * 100f) / segmentsTotal).toInt().coerceIn(0, 100)
            totalBytes != null && totalBytes > 0L -> ((bytesDownloaded * 100L) / totalBytes).toInt().coerceIn(0, 100)
            else -> null
        }

        val speedText = if (lastSpeedBps > 0L) " · ${formatBytes(lastSpeedBps)}/s" else ""
        val queueSuffix = if (active.size > 1) " (+${active.size - 1} queued)" else ""
        val progressDetail = when {
            totalBytes != null && totalBytes > 0L -> "${formatBytes(bytesDownloaded)} / ${formatBytes(totalBytes)}$speedText"
            segmentsTotal > 0 -> "$segmentsDone/$segmentsTotal segments$speedText"
            else -> "${formatBytes(bytesDownloaded)}$speedText"
        }

        return buildNotification(
            title = "${current.title}$queueSuffix",
            text = progressDetail,
            percent = percent,
        )
    }

    private fun buildNotification(
        title: String,
        text: String,
        percent: Int?,
    ): Notification {
        val openAppIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val pauseAllIntent = PendingIntent.getService(
            this,
            1,
            Intent(this, DownloadForegroundService::class.java).apply {
                action = ACTION_PAUSE_ALL
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

        val builder = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setContentTitle(title)
            .setContentText(text)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setContentIntent(openAppIntent)
            .addAction(android.R.drawable.ic_media_pause, "Pause All", pauseAllIntent)
            .setPriority(NotificationCompat.PRIORITY_LOW)

        if (percent != null) {
            builder.setProgress(100, percent, false)
        } else {
            builder.setProgress(0, 0, true)
        }
        return builder.build()
    }

    private fun startForegroundCompat(notification: Notification) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    private fun stopSelfSafely() {
        observerJob?.cancel()
        observerJob = null
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            stopForeground(STOP_FOREGROUND_REMOVE)
        } else {
            @Suppress("DEPRECATION")
            stopForeground(true)
        }
        stopSelf()
    }

    private fun ensureChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val mgr = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (mgr.getNotificationChannel(CHANNEL_ID) != null) return
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Offline Downloads",
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = "Shows progress for active Vessel offline media downloads"
            setShowBadge(false)
        }
        mgr.createNotificationChannel(channel)
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        observerJob?.cancel()
        serviceScope.cancel()
        super.onDestroy()
    }

    private fun formatBytes(bytes: Long): String {
        if (bytes <= 0L) return "0 B"
        val units = arrayOf("B", "KB", "MB", "GB")
        var value = bytes.toDouble()
        var idx = 0
        while (value >= 1024.0 && idx < units.lastIndex) {
            value /= 1024.0
            idx++
        }
        return if (idx == 0) "$bytes B" else String.format("%.1f %s", value, units[idx])
    }

    companion object {
        private const val CHANNEL_ID = "vessel_downloads"
        private const val NOTIFICATION_ID = 4401
        private const val ACTION_PAUSE_ALL = "dev.falsis.vessel.action.PAUSE_ALL_DOWNLOADS"
        private const val ACTION_STOP = "dev.falsis.vessel.action.STOP_DOWNLOAD_SERVICE"

        fun ensureRunning(context: Context) {
            val intent = Intent(context, DownloadForegroundService::class.java)
            ContextCompat.startForegroundService(context, intent)
        }
    }
}
