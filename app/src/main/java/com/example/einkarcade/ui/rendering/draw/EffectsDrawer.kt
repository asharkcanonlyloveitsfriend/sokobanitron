package com.example.einkarcade.ui.rendering.draw

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.PorterDuff
import android.graphics.PorterDuffColorFilter
import androidx.core.graphics.withClip
import androidx.core.graphics.withTranslation
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.AndroidGameAssets
import com.example.einkarcade.ui.rendering.RenderTimings
import com.example.einkarcade.ui.rendering.VanishSpec
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import com.example.einkarcade.ui.rendering.geom.snapToWholePixel
import com.example.einkarcade.ui.rendering.geom.spriteDrawParams
import com.example.einkarcade.ui.rendering.geom.toRenderPoint

internal data class OverlayState(
    val boxPathActive: Boolean,
    val boxPathSuppressLine: Boolean,
    val boxPath: List<Position>,
    val boxPathShrink: Float,
    val boxPathStartMs: Long,
    val vanishPosition: Position?,
    val vanishStep: Int?,
    val boxFlashPosition: Position?,
    val boxFlashStartMs: Long,
    val playerSilhouettePosition: Position?,
    val playerSilhouetteStartMs: Long,
    val playerFlashPosition: Position?,
    val playerFlashStartMs: Long,
    val blinkActive: Boolean
)

internal class EffectsDrawer(private val assets: AndroidGameAssets) {
    private val boxPathPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFD3D3D3.toInt()
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
    }
    private val boxPathTailPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFF2F2F2.toInt()
        style = Paint.Style.FILL
    }
    private val boxPathDotPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0xFFD3D3D3.toInt()
        style = Paint.Style.FILL
    }
    private val playerSilhouetteDarkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFF8E8E8E.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val playerSilhouetteLightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFFF2F2F2.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val playerFlashDarkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFF8E8E8E.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val playerFlashLightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFFF2F2F2.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val boxFlashDarkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFF8E8E8E.toInt(), PorterDuff.Mode.SRC_IN)
    }
    private val boxFlashLightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        colorFilter = PorterDuffColorFilter(0xFFF2F2F2.toInt(), PorterDuff.Mode.SRC_IN)
    }

    fun drawBoxPathLine(canvas: Canvas, viewport: BoardViewport, overlay: OverlayState, nowMs: Long) {
        if (!overlay.boxPathActive) return
        if (overlay.boxPath.size < 2) return
        if (overlay.boxPathSuppressLine) return
        if (nowMs - overlay.boxPathStartMs < RenderTimings.BOX_PATH_DELAY_MS) return
        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY
        val strokeWidth = cellSize * 0.2f
        boxPathPaint.strokeWidth = strokeWidth
        boxPathTailPaint.strokeWidth = strokeWidth

        val points = overlay.boxPath.map { position ->
            val cx = offsetX + (position.col + 1) * cellSize + cellSize / 2f
            val cy = offsetY + (position.row + 1) * cellSize + cellSize / 2f
            android.graphics.PointF(cx, cy)
        }

        val totalSegments = points.size - 1
        val startT = totalSegments.toFloat() * overlay.boxPathShrink.coerceIn(0f, 1f)
        val startSegment = startT.toInt().coerceIn(0, totalSegments - 1)
        val startFraction = startT - startSegment

        fun interpolate(start: android.graphics.PointF, end: android.graphics.PointF, t: Float): android.graphics.PointF {
            return android.graphics.PointF(
                start.x + (end.x - start.x) * t,
                start.y + (end.y - start.y) * t
            )
        }

        val startPoint = interpolate(points[startSegment], points[startSegment + 1], startFraction)

        val tailParams = spriteDrawParams(viewport, overlay.boxPath[startSegment], 0.90f)
        val tailOpaque = assets.getOpaqueBounds(R.drawable.box, tailParams.sizePx)
        if (!tailOpaque.isEmpty) {
            val left = tailParams.left + tailOpaque.left
            val top = tailParams.top + tailOpaque.top
            val right = tailParams.left + tailOpaque.right
            val bottom = tailParams.top + tailOpaque.bottom
            canvas.drawRect(left, top, right, bottom, boxPathTailPaint)
        }

        var prev = startPoint
        var drewAnySegment = false
        for (index in (startSegment + 1) until points.size) {
            val next = points[index]
            canvas.drawLine(prev.x, prev.y, next.x, next.y, boxPathPaint)
            prev = next
            drewAnySegment = true
        }

        if (!drewAnySegment) {
            canvas.drawCircle(startPoint.x, startPoint.y, strokeWidth / 2f, boxPathDotPaint)
        }
    }

    fun drawVanishingBox(canvas: Canvas, viewport: BoardViewport, overlay: OverlayState) {
        val currentPosition = overlay.vanishPosition ?: return
        val step = overlay.vanishStep ?: return
        require(step in 0..VanishSpec.LAST_STEP) { "Vanish step out of range: $step" }

        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY

        val paddedPosition = Position(currentPosition.row + 1, currentPosition.col + 1)
        val origin = paddedPosition.toRenderPoint(cellSize, offsetX, offsetY)
        val targetSize = snapToWholePixel(cellSize * 0.90f)
        val sizePx = targetSize.toInt()
        require(sizePx > 0)
        val leftPx = snapToWholePixel(origin.x + (cellSize - targetSize) / 2f)
        val topPx = snapToWholePixel(origin.y + (cellSize - targetSize) / 2f)
        val scale = VanishSpec.scale(step)
        val size = targetSize * scale
        if (size <= 0f) return
        val left = leftPx + (targetSize - size) / 2f
        val top = topPx + (targetSize - size) / 2f
        val bitmap = assets.getBitmap(R.drawable.box, sizePx)
        canvas.withClip(left, top, left + size, top + size) {
            drawBitmap(bitmap, leftPx, topPx, assets.bitmapPaint())
        }
    }

    fun drawBoxFlash(
        canvas: Canvas,
        viewport: BoardViewport,
        overlay: OverlayState,
        nowMs: Long
    ) {
        if (overlay.boxFlashPosition == null) return
        val elapsedMs = nowMs - overlay.boxFlashStartMs
        if (elapsedMs > RenderTimings.FLASH_DURATION_MS) return
        val params = spriteDrawParams(viewport, overlay.boxFlashPosition, 0.90f)
        val bitmap = assets.getBitmap(R.drawable.box, params.sizePx)
        drawFlashedBitmap(
            canvas = canvas,
            bitmap = bitmap,
            left = params.left,
            top = params.top,
            elapsedMs = elapsedMs,
            darkPaint = boxFlashDarkPaint,
            lightPaint = boxFlashLightPaint
        )
    }

    fun drawPlayerSilhouette(
        canvas: Canvas,
        viewport: BoardViewport,
        overlay: OverlayState,
        nowMs: Long,
        isFacingLeft: Boolean
    ) {
        if (!overlay.boxPathActive || overlay.playerSilhouettePosition == null) return
        val elapsedMs = nowMs - overlay.playerSilhouetteStartMs
        if (elapsedMs > RenderTimings.FLASH_DURATION_MS) return
        val params = spriteDrawParams(viewport, overlay.playerSilhouettePosition, 0.80f)
        val body = assets.getBitmap(R.drawable.player_slime, params.sizePx)
        drawFlashedSprite(
            canvas = canvas,
            bitmap = body,
            left = params.left,
            top = params.top,
            sizePx = params.sizePx,
            flipX = isFacingLeft,
            elapsedMs = elapsedMs,
            darkPaint = playerSilhouetteDarkPaint,
            lightPaint = playerSilhouetteLightPaint
        )
    }

    fun drawPlayerFlash(
        canvas: Canvas,
        viewport: BoardViewport,
        overlay: OverlayState,
        nowMs: Long,
        isFacingLeft: Boolean
    ) {
        if (overlay.boxPathActive || overlay.playerFlashPosition == null) return
        val elapsedMs = nowMs - overlay.playerFlashStartMs
        if (elapsedMs > RenderTimings.FLASH_DURATION_MS) return
        val params = spriteDrawParams(viewport, overlay.playerFlashPosition, 0.80f)
        val body = assets.getBitmap(R.drawable.player_slime, params.sizePx)
        drawFlashedSprite(
            canvas = canvas,
            bitmap = body,
            left = params.left,
            top = params.top,
            sizePx = params.sizePx,
            flipX = isFacingLeft,
            elapsedMs = elapsedMs,
            darkPaint = playerFlashDarkPaint,
            lightPaint = playerFlashLightPaint
        )
    }
}

internal const val FLASH_PHASE_MS: Long = 50L

internal fun drawSprite(
    canvas: Canvas,
    bitmap: Bitmap,
    left: Float,
    top: Float,
    sizePx: Int,
    flipX: Boolean,
    paint: Paint
) {
    canvas.withTranslation(left, top) {
        if (flipX) {
            scale(-1f, 1f, sizePx / 2f, sizePx / 2f)
        }
        drawBitmap(bitmap, 0f, 0f, paint)
    }
}

internal fun drawFlashedSprite(
    canvas: Canvas,
    bitmap: Bitmap,
    left: Float,
    top: Float,
    sizePx: Int,
    flipX: Boolean,
    elapsedMs: Long,
    darkPaint: Paint,
    lightPaint: Paint
) {
    val paint = if (elapsedMs <= FLASH_PHASE_MS) darkPaint else lightPaint
    drawSprite(canvas, bitmap, left, top, sizePx, flipX, paint)
}

internal fun drawFlashedBitmap(
    canvas: Canvas,
    bitmap: Bitmap,
    left: Float,
    top: Float,
    elapsedMs: Long,
    darkPaint: Paint,
    lightPaint: Paint
) {
    val paint = if (elapsedMs <= FLASH_PHASE_MS) darkPaint else lightPaint
    canvas.drawBitmap(bitmap, left, top, paint)
}
