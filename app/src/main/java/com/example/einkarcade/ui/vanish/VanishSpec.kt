package com.example.einkarcade.ui.vanish

/**
 * Single source of truth for vanish step timing and step count.
 *
 * Steps are 0..LAST_STEP inclusive.
 */
internal object VanishSpec {
    const val VANISH_BASE_DELAY_MS: Long = 170L

    const val LAST_STEP: Int = 4
    const val TOTAL_STEPS: Int = LAST_STEP + 1

    fun delayMs(step: Int): Long = when (step) {
        0 -> VANISH_BASE_DELAY_MS
        1 -> (VANISH_BASE_DELAY_MS * 0.75f).toLong()
        2 -> (VANISH_BASE_DELAY_MS * 0.5f).toLong()
        3 -> (VANISH_BASE_DELAY_MS * 0.36f).toLong()
        4 -> (VANISH_BASE_DELAY_MS * 0.2f).toLong()
        else -> 0L
    }
}