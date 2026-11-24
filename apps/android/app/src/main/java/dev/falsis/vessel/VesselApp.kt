package dev.falsis.vessel

import android.app.Application
import dev.falsis.vessel.data.VesselRepository
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

class VesselApp : Application() {
    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onCreate() {
        super.onCreate()
        val repo = VesselRepository.get(this)
        appScope.launch {
            repo.initialize()
        }
    }
}
