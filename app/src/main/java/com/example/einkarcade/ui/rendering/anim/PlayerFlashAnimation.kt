package com.example.einkarcade.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.PorterDuff
import android.graphics.PorterDuffColorFilter
import android.graphics.Rect
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.draw.GameRenderer
import com.example.einkarcade.ui.rendering.geom.BoardViewport

/**
 * Two-phase player flash: dark then light, then cleanup.
 */
internal class PlayerFlashAnimation(
    private val renderer: GameRenderer,
    private val viewport: BoardViewport,
    private val position: Position
) : Animation {

    private val spriteRect: Rect by lazy { renderer.computePlayerRect(viewport, position) }
    private val bodyBitmap by lazy { renderer.getPlayerBodyBitmap() }

    private val darkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFF8E8E8E.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val lightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFFF2F2F2.toInt(), PorterDuff.Mode.SRC_IN)
    }

    private enum class Phase {
        DARK,
        LIGHT,
        CLEANUP
    }

    private var phase: Phase = Phase.DARK

    override fun dirtyRect(): Rect? {
        return spriteRect
    }

    override fun drawOverEntities(canvas: Canvas) {
        when (phase) {
            Phase.DARK -> {
                canvas.drawBitmap(bodyBitmap, null, spriteRect, darkPaint)
                phase = Phase.LIGHT
            }
            Phase.LIGHT -> {
                canvas.drawBitmap(bodyBitmap, null, spriteRect, lightPaint)
                phase = Phase.CLEANUP
            }
            Phase.CLEANUP -> {}
        }
    }

    override fun ticksUntilNextStep(): Int? {
        return when (phase) {
            Phase.DARK -> 2
            Phase.LIGHT -> 1
            Phase.CLEANUP -> null
        }
    }
}
