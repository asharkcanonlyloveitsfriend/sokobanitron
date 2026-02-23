package com.example.einkarcade.ui.rendering.anim

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.PointF
import android.graphics.Rect
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import kotlin.math.ceil
import kotlin.math.floor
import kotlin.math.pow

private const val SPEED_SCALE = 1.3f // overall speed multiplier
private const val SPEED_EXPONENT = 0.5f // nonlinear acceleration factor (0 < exponent < 1)

internal class BoxPathAnimation(
    private val viewport: BoardViewport,
    private val path: List<Position>,
) : Animation {
    private val pathRect: Rect by lazy { computePathRect() }
    private val pathPaint =
        Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = 0xFFD3D3D3.toInt()
            style = Paint.Style.STROKE
            strokeCap = Paint.Cap.ROUND
            strokeJoin = Paint.Join.ROUND
        }

    private var pathProgressSegments: Float = 0f
    private var isComplete: Boolean = false

    private val totalSegments: Int = totalPathSegments()

    private val points: List<PointF> by lazy {
        path.map { position ->
            val cx =
                viewport.offsetX + (position.col + 1) * viewport.cellSize + viewport.cellSize / 2f
            val cy =
                viewport.offsetY + (position.row + 1) * viewport.cellSize + viewport.cellSize / 2f
            PointF(cx, cy)
        }
    }

    private val speedPerTick: Float

    init {
        val total = totalSegments.toFloat()
        speedPerTick = SPEED_SCALE * total.pow(SPEED_EXPONENT)
        pathPaint.strokeWidth = viewport.cellSize * 0.2f
    }

    override fun dirtyRects(): Array<Rect?> = arrayOf(pathRect)

    override fun drawUnderEntities(canvas: Canvas) {
        if (isComplete) return
        drawPath(canvas)
        pathProgressSegments += speedPerTick

        if (pathProgressSegments >= totalSegments) {
            isComplete = true
        }
    }

    override fun ticksUntilNextStep(): Int? = if (isComplete) null else 1

    override fun hidesPlayer(): Boolean = true

    private fun drawPath(canvas: Canvas) {
        if (points.size < 2) return

        val consumed = minOf(pathProgressSegments, totalSegments.toFloat())
        val startSegment = consumed.toInt().coerceIn(0, totalSegments - 1)
        val startFraction = consumed - startSegment

        fun interpolate(
            start: PointF,
            end: PointF,
            t: Float,
        ): PointF =
            PointF(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t,
            )

        val startPoint = interpolate(points[startSegment], points[startSegment + 1], startFraction)
        var prev = startPoint
        for (index in (startSegment + 1) until points.size) {
            val next = points[index]
            canvas.drawLine(prev.x, prev.y, next.x, next.y, pathPaint)
            prev = next
        }
    }

    private fun totalPathSegments(): Int = (path.size - 1).coerceAtLeast(0)

    private fun computePathRect(): Rect {
        if (points.isEmpty()) return Rect()
        val halfStroke = pathPaint.strokeWidth / 2f
        val minX = points.minOf { it.x } - halfStroke
        val minY = points.minOf { it.y } - halfStroke
        val maxX = points.maxOf { it.x } + halfStroke
        val maxY = points.maxOf { it.y } + halfStroke
        return Rect(
            floor(minX).toInt(),
            floor(minY).toInt(),
            ceil(maxX).toInt(),
            ceil(maxY).toInt(),
        )
    }
}
