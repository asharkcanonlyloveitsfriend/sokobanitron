package com.example.einkarcade.ui.rendering

import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.painter.Painter
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.screens.VanishState
import com.example.einkarcade.ui.vanish.VanishSpec
import kotlin.math.roundToInt

internal fun DrawScope.drawVanishingBox(
    vanish: VanishState?,
    gridPosition: Position,
    paddedPosition: Position,
    boxPainter: Painter,
    selectedBoxPainter: Painter,
    cellSize: Float,
    offsetX: Float,
    offsetY: Float
) {
    if (vanish == null || vanish.position != gridPosition) return
    require(vanish.step in 0..VanishSpec.LAST_STEP) { "Vanish step out of range: ${vanish.step}" }

    // Steps 0..LAST_STEP all render using the same geometry; per-step differences are handled below.
    run {
        val tileLeft = offsetX + paddedPosition.col * cellSize
        val tileTop = offsetY + paddedPosition.row * cellSize
        val baseSize = (cellSize * 0.90f * 0.72f).roundToInt().toFloat()
        val baseLeft = (tileLeft + (cellSize - baseSize) / 2).roundToInt().toFloat()
        val baseTop = (tileTop + (cellSize - baseSize) / 2).roundToInt().toFloat()
        val scale = when (vanish.step) {
            0 -> 1.0f
            1 -> 0.75f
            2 -> 0.5f
            3 -> 0.3f
            else -> 0.18f
        }
        val size = baseSize * scale
        val left = baseLeft + (baseSize - size) / 2
        val top = baseTop + (baseSize - size) / 2
        val innerRadius = size * (14f / 72f)

        val shade = when (vanish.step) {
            0 -> Color(0xFF6B7280)
            1 -> Color(0xFF646C79)
            2 -> Color(0xFF5E6672)
            else -> Color(0xFF58616C)
        }

        if (vanish.step == 0) {
            drawBox(
                paddedPosition,
                boxPainter,
                selectedBoxPainter,
                false,
                cellSize,
                offsetX,
                offsetY
            )
        } else {
            drawRoundRect(
                color = shade,
                topLeft = Offset(left, top),
                size = Size(size, size),
                cornerRadius = CornerRadius(innerRadius, innerRadius)
            )
        }
    }
}
