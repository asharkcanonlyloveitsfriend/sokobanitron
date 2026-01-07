package com.example.einkarcade.ui.rendering

/**
 * SurfaceView-only vanish timing + geometry.
 *
 * Steps are 0..LAST_STEP inclusive.
 */
internal object VanishSpec {
    const val VANISH_BASE_DELAY_MS: Long = 255L

    const val LAST_STEP: Int = 6
    const val TOTAL_STEPS: Int = LAST_STEP + 1

    fun delayMs(step: Int): Long = when (step) {
        0 -> VANISH_BASE_DELAY_MS
        1 -> (VANISH_BASE_DELAY_MS * 0.75f).toLong()
        2 -> (VANISH_BASE_DELAY_MS * 0.5f).toLong()
        3 -> (VANISH_BASE_DELAY_MS * 0.36f).toLong()
        4 -> (VANISH_BASE_DELAY_MS * 0.2f).toLong()
        5 -> (VANISH_BASE_DELAY_MS * 0.14f).toLong()
        6 -> (VANISH_BASE_DELAY_MS * 0.1f).toLong()
        else -> 0L
    }

    fun scale(step: Int): Float = when (step) {
        0 -> 1.0f
        1 -> 0.75f
        2 -> 0.5f
        3 -> 0.3f
        4 -> 0.18f
        5 -> 0.14f
        6 -> 0.1f
        else -> error("Unsupported vanish step: $step")
    }
}
