package com.sokobanitron.app.dev

import android.view.Surface

object NativeBridge {
    private const val LIB_NAME = "sokobanitron_android_jni"

    @Volatile
    private var loadAttempted = false

    @Volatile
    private var loaded = false

    fun create(
        levelSetsRoot: String,
        surfaceWidth: Int,
        surfaceHeight: Int,
    ): Long {
        check(ensureLoaded()) { "Native library '$LIB_NAME' is not loaded." }
        val handle = nativeCreate(levelSetsRoot, surfaceWidth, surfaceHeight)
        check(handle != 0L) { "Failed to create native Android client handle." }
        return handle
    }

    fun destroy(handle: Long) {
        if (!ensureLoaded() || handle == 0L) return
        nativeDestroy(handle)
    }

    fun resize(
        handle: Long,
        surfaceWidth: Int,
        surfaceHeight: Int,
    ) {
        check(ensureLoaded()) { "Native library '$LIB_NAME' is not loaded." }
        nativeResize(handle, surfaceWidth, surfaceHeight)
    }

    fun setSurface(
        handle: Long,
        surface: Surface?,
    ) {
        check(ensureLoaded()) { "Native library '$LIB_NAME' is not loaded." }
        nativeSetSurface(handle, surface)
    }

    fun onPointerEvent(
        handle: Long,
        pointerId: Long,
        phase: Int,
        x: Float,
        y: Float,
    ): Boolean {
        check(ensureLoaded()) { "Native library '$LIB_NAME' is not loaded." }
        return nativeOnPointerEvent(handle, pointerId, phase, x, y)
    }

    fun presentFrame(handle: Long): Boolean {
        check(ensureLoaded()) { "Native library '$LIB_NAME' is not loaded." }
        return nativePresentFrame(handle)
    }

    fun hasPendingGameplayPresentation(handle: Long): Boolean {
        check(ensureLoaded()) { "Native library '$LIB_NAME' is not loaded." }
        return nativeHasPendingGameplayPresentation(handle)
    }

    private fun ensureLoaded(): Boolean {
        if (loadAttempted) return loaded
        synchronized(this) {
            if (loadAttempted) return loaded
            loadAttempted = true
            loaded =
                try {
                    System.loadLibrary(LIB_NAME)
                    true
                } catch (_: UnsatisfiedLinkError) {
                    false
                } catch (_: SecurityException) {
                    false
                }
            return loaded
        }
    }

    private external fun nativeCreate(
        levelSetsRoot: String,
        surfaceWidth: Int,
        surfaceHeight: Int,
    ): Long

    private external fun nativeDestroy(handle: Long)

    private external fun nativeResize(
        handle: Long,
        surfaceWidth: Int,
        surfaceHeight: Int,
    )

    private external fun nativeSetSurface(
        handle: Long,
        surface: Surface?,
    )

    private external fun nativeOnPointerEvent(
        handle: Long,
        pointerId: Long,
        phase: Int,
        x: Float,
        y: Float,
    ): Boolean

    private external fun nativePresentFrame(handle: Long): Boolean

    private external fun nativeHasPendingGameplayPresentation(handle: Long): Boolean
}
