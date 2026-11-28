package dev.falsis.vessel.data

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import uniffi.vessel_ffi.NativeSecretBridge
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Hardware/OS-backed Android Keystore secret bridge for UniFFI (`SecretStore`).
 *
 * Encrypts each secret individually with AES-256-GCM using a key stored in
 * `AndroidKeyStore` and persists the IV + ciphertext in private `SharedPreferences`.
 */
class AndroidSecretBridge(context: Context) : NativeSecretBridge {
    private val prefs = context.applicationContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
    private val keyAlias = KEY_ALIAS
    private val lock = Any()

    override fun getSecret(key: String): String? {
        val trimmed = key.trim()
        if (trimmed.isEmpty()) return null
        synchronized(lock) {
            val encoded = prefs.getString(trimmed, null) ?: return null
            val parts = encoded.split(':', limit = 2)
            if (parts.size != 2) return null
            return runCatching {
                val iv = Base64.decode(parts[0], Base64.NO_WRAP)
                val cipherBytes = Base64.decode(parts[1], Base64.NO_WRAP)
                val cipher = Cipher.getInstance(TRANSFORMATION)
                cipher.init(Cipher.DECRYPT_MODE, getOrCreateSecretKey(), GCMParameterSpec(GCM_TAG_BITS, iv))
                String(cipher.doFinal(cipherBytes), Charsets.UTF_8)
            }.getOrNull()
        }
    }

    override fun setSecret(key: String, value: String) {
        val trimmed = key.trim()
        require(trimmed.isNotEmpty()) { "[VesselInvalidInputError]: secret key cannot be empty" }
        synchronized(lock) {
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, getOrCreateSecretKey())
            val iv = cipher.iv
            val cipherBytes = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
            val payload = "${Base64.encodeToString(iv, Base64.NO_WRAP)}:${Base64.encodeToString(cipherBytes, Base64.NO_WRAP)}"
            check(prefs.edit().putString(trimmed, payload).commit()) {
                "[VesselStorageError]: failed committing encrypted Android secret"
            }
        }
    }

    override fun deleteSecret(key: String) {
        val trimmed = key.trim()
        if (trimmed.isEmpty()) return
        synchronized(lock) {
            check(prefs.edit().remove(trimmed).commit()) {
                "[VesselStorageError]: failed removing encrypted Android secret"
            }
        }
    }

    override fun listSecretKeys(): List<String> = synchronized(lock) {
        prefs.all.keys.sorted()
    }

    private fun getOrCreateSecretKey(): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        val existing = (keyStore.getEntry(keyAlias, null) as? KeyStore.SecretKeyEntry)?.secretKey
        if (existing != null) return existing

        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        val spec = KeyGenParameterSpec.Builder(
            keyAlias,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setRandomizedEncryptionRequired(true)
            .build()
        generator.init(spec)
        return generator.generateKey()
    }

    private companion object {
        const val ANDROID_KEYSTORE = "AndroidKeyStore"
        const val PREFS_NAME = "vessel_keystore_secrets"
        const val KEY_ALIAS = "vessel_secrets_aes_256_gcm_v1"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val GCM_TAG_BITS = 128
    }
}
