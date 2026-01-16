package com.example.einkarcade.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.PointF
import android.graphics.PorterDuff
import android.graphics.PorterDuffColorFilter
import android.graphics.Rect
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.draw.GameRenderer
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import kotlin.math.ceil
import kotlin.math.floor

private const val FLASH_LIGHT_TICKS = 1
private const val FLASH_DARK_TICKS = 1
// Number of path segments consumed per tick (may be fractional)
private const val PATH_SEGMENTS_PER_TICK = 2.6f

internal class BoxMoveAnimation(
    private val renderer: GameRenderer,
    private val viewport: BoardViewport,
    private val playerFrom: Position,
    private val path: List<Position>
) : Animation {

    private val boxFrom: Position = path.first()
    private val playerRect: Rect by lazy { renderer.computePlayerRect(viewport, playerFrom) }
    private val boxRect: Rect by lazy { renderer.computeBoxRect(viewport, boxFrom) }
    private val pathRect: Rect by lazy { computePathRect() }

    private val darkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFF8E8E8E.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val lightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFFF2F2F2.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val pathPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFD3D3D3.toInt()
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
    }

    private enum class Phase {
        FLASH_LIGHT,
        FLASH_DARK,
        PATH,
        CLEANUP
    }

    private var phase: Phase = Phase.FLASH_LIGHT
    private var pathProgressSegments: Float = 0f

    override fun dirtyRect(): Rect {
        return Rect(pathRect).apply {
            union(playerRect)
            union(boxRect)
        }
    }

    override fun drawUnderEntities(canvas: Canvas) {
        if (phase == Phase.PATH) {
            drawPath(canvas)
            pathProgressSegments += PATH_SEGMENTS_PER_TICK
            if (pathProgressSegments >= totalPathSegments()) {
                phase = Phase.CLEANUP
            }
        }
    }

    override fun drawOverEntities(canvas: Canvas) {
        when (phase) {
            Phase.FLASH_LIGHT -> {
                drawFlashes(canvas, lightPaint)
                phase = Phase.FLASH_DARK
            }
            Phase.FLASH_DARK -> {
                drawFlashes(canvas, darkPaint)
                phase = Phase.PATH
            }
            Phase.PATH,
            Phase.CLEANUP -> {}
        }
    }

    override fun ticksUntilNextStep(): Int? {
        return when (phase) {
            Phase.FLASH_LIGHT -> FLASH_LIGHT_TICKS
            Phase.FLASH_DARK -> FLASH_DARK_TICKS
            Phase.PATH -> 1
            Phase.CLEANUP -> null
        }
    }

    private fun drawFlashes(canvas: Canvas, paint: Paint) {
        canvas.drawBitmap(renderer.getPlayerBodyBitmap(), null, playerRect, paint)
        canvas.drawBitmap(renderer.getBoxBitmap(), null, boxRect, paint)
    }

    private fun drawPath(canvas: Canvas) {
        if (path.size < 2) return
        val points = path.map { position ->
            val cx = viewport.offsetX + (position.col + 1) * viewport.cellSize + viewport.cellSize / 2f
            val cy = viewport.offsetY + (position.row + 1) * viewport.cellSize + viewport.cellSize / 2f
            PointF(cx, cy)
        }
        val totalSegments = points.size - 1
        val consumed = pathProgressSegments.coerceIn(0f, totalSegments.toFloat())
        val startSegment = consumed.toInt().coerceIn(0, totalSegments - 1)
        val startFraction = consumed - startSegment

        fun interpolate(start: PointF, end: PointF, t: Float): PointF {
            return PointF(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t
            )
        }

        val startPoint = interpolate(points[startSegment], points[startSegment + 1], startFraction)
        pathPaint.strokeWidth = viewport.cellSize * 0.2f
        var prev = startPoint
        for (index in (startSegment + 1) until points.size) {
            val next = points[index]
            canvas.drawLine(prev.x, prev.y, next.x, next.y, pathPaint)
            prev = next
        }
    }

    private fun totalPathSegments(): Int {
        return (path.size - 1).coerceAtLeast(0)
    }

    private fun computePathRect(): Rect {
        if (path.isEmpty()) return Rect()
        val strokeWidth = viewport.cellSize * 0.2f
        val halfStroke = strokeWidth / 2f
        val points = path.map { position ->
            val cx = viewport.offsetX + (position.col + 1) * viewport.cellSize + viewport.cellSize / 2f
            val cy = viewport.offsetY + (position.row + 1) * viewport.cellSize + viewport.cellSize / 2f
            PointF(cx, cy)
        }
        val minX = points.minOf { it.x } - halfStroke
        val minY = points.minOf { it.y } - halfStroke
        val maxX = points.maxOf { it.x } + halfStroke
        val maxY = points.maxOf { it.y } + halfStroke
        return Rect(
            floor(minX).toInt(),
            floor(minY).toInt(),
            ceil(maxX).toInt(),
            ceil(maxY).toInt()
        )
    }

    override fun hidesPlayer(): Boolean = true
}
