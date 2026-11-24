package dev.falsis.vessel

import android.app.Application
import android.content.Context
import android.util.Log
import dev.falsis.vessel.data.VesselRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

class VesselApp : Application() {
    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    private external fun nativeInitTls(context: Context)

    override fun onCreate() {
        super.onCreate()
        runCatching {
            System.loadLibrary("vessel_ffi")
            nativeInitTls(this)
            Log.i("VesselApp", "nativeInitTls initialized")
        }.onFailure { err ->
            Log.e("VesselApp", "nativeInitTls failed", err)
        }
        val repo = VesselRepository.get(this)
        appScope.launch {
            repo.initialize()
        }
    }
}
