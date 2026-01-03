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
import com.example.einkarcade.ui.vanish.VanishVisualSpec
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
        val tileLeft = offsetX + paddedPosition.col * cellSize
        val tileTop = offsetY + paddedPosition.row * cellSize
        val baseSize = (cellSize * VanishVisualSpec.BASE_SIZE_FACTOR).roundToInt().toFloat()
        val baseLeft = (tileLeft + (cellSize - baseSize) / 2).roundToInt().toFloat()
        val baseTop = (tileTop + (cellSize - baseSize) / 2).roundToInt().toFloat()
        val scale = VanishVisualSpec.scale(vanish.step)
        val size = baseSize * scale
        val left = baseLeft + (baseSize - size) / 2
        val top = baseTop + (baseSize - size) / 2
        val innerRadius = size * VanishVisualSpec.CORNER_RADIUS_FACTOR
        if (VanishVisualSpec.isSpriteStep(vanish.step)) {
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
            val shade = Color(VanishVisualSpec.colorArgb(vanish.step))
            drawRoundRect(
                color = shade,
                topLeft = Offset(left, top),
                size = Size(size, size),
                cornerRadius = CornerRadius(innerRadius, innerRadius)
            )
        }
}
