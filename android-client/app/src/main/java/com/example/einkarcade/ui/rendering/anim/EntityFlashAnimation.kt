package com.example.einkarcade.ui.rendering.anim

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.PorterDuff
import android.graphics.PorterDuffColorFilter
import android.graphics.Rect
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.draw.EntityRenderer
import com.example.einkarcade.ui.rendering.geom.BoardViewport

private const val FLASH_LIGHT_TICKS = 1
private const val FLASH_DARK_TICKS = 1

internal class EntityFlashAnimation(
    private val renderer: EntityRenderer,
    private val viewport: BoardViewport,
    private val playerPosition: Position,
    private val boxPositions: List<Position>,
) : Animation {
    private val darkPaint =
        Paint(Paint.ANTI_ALIAS_FLAG).apply {
            colorFilter = PorterDuffColorFilter(0xFF8E8E8E.toInt(), PorterDuff.Mode.SRC_IN)
        }
    private val lightPaint =
        Paint(Paint.ANTI_ALIAS_FLAG).apply {
            colorFilter = PorterDuffColorFilter(0xFFF2F2F2.toInt(), PorterDuff.Mode.SRC_IN)
        }

    private val playerBitmap by lazy { renderer.getPlayerBodyBitmap() }
    private val boxBitmap by lazy { renderer.getBoxBitmap() }
    private val drawItems: List<DrawItem> = buildDrawItems()
    private var phase: Phase = Phase.FLASH_DARK

    override fun dirtyRects(): Array<Rect?> = drawItems.map { it.rect }.toTypedArray()

    override fun drawOverEntities(canvas: Canvas) {
        when (phase) {
            Phase.FLASH_DARK -> {
                drawFlashes(canvas, darkPaint)
                phase = Phase.FLASH_LIGHT
            }

            Phase.FLASH_LIGHT -> {
                drawFlashes(canvas, lightPaint)
                phase = Phase.CLEANUP
            }

            Phase.CLEANUP -> {}
        }
    }

    override fun ticksUntilNextStep(): Int? =
        when (phase) {
            Phase.FLASH_DARK -> FLASH_DARK_TICKS
            Phase.FLASH_LIGHT -> FLASH_LIGHT_TICKS
            Phase.CLEANUP -> null
        }

    override fun hidesPlayer(): Boolean = true

    private enum class Phase {
        FLASH_DARK,
        FLASH_LIGHT,
        CLEANUP,
    }

    private fun drawFlashes(
        canvas: Canvas,
        paint: Paint,
    ) {
        for (item in drawItems) {
            canvas.drawBitmap(item.bitmap, null, item.rect, paint)
        }
    }

    private fun buildDrawItems(): List<DrawItem> {
        val items = mutableListOf<DrawItem>()
        items.add(DrawItem(playerBitmap, renderer.computePlayerRect(viewport, playerPosition)))
        for (position in boxPositions) {
            items.add(DrawItem(boxBitmap, renderer.computeBoxRect(viewport, position)))
        }
        return items
    }

    private data class DrawItem(
        val bitmap: Bitmap,
        val rect: Rect,
    )
}
