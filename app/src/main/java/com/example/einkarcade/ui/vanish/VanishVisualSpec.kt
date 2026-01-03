package com.example.einkarcade.ui.vanish

internal object VanishVisualSpec {
    fun isSpriteStep(step: Int): Boolean = (step == 0)

    fun scale(step: Int): Float = when (step) {
        0 -> 1.0f
        1 -> 0.75f
        2 -> 0.5f
        3 -> 0.3f
        4 -> 0.18f
        5 -> 0.1f
        else -> error("Unsupported vanish step: $step")
    }

    fun colorArgb(step: Int): Int = when (step) {
        1 -> 0xFF646C79.toInt()
        2 -> 0xFF5E6672.toInt()
        3 -> 0xFF58616C.toInt()
        4 -> 0xFF58616C.toInt()
        5 -> 0xFF58616C.toInt()
        else -> error("No rect color for vanish step: $step")
    }

    const val BASE_SIZE_FACTOR: Float = 0.90f * 0.72f
    const val CORNER_RADIUS_FACTOR: Float = 14f / 72f
}
