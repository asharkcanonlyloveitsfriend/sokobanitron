package com.example.einkarcade.ui.rendering.anim

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.ColorMatrixColorFilter
import android.graphics.Paint
import android.graphics.PorterDuff
import android.graphics.PorterDuffColorFilter
import android.graphics.Rect
import android.graphics.Region
import android.graphics.RegionIterator
import androidx.core.graphics.withSave
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import kotlin.math.max
import kotlin.math.min

private const val TICKS_PER_STEP = 3
private const val STEP_PERCENT = 8   // percent of union rect width per step
private const val FLASH_GAP_STEPS = 6 // how many sweep steps to wait after the band passes a tile
private const val FLASH_COUNT = 2
private const val TILE_FLASH_PHASES = 2 * FLASH_COUNT - 1

internal class LevelTransitionAnimation(
    private val oldBitmap: Bitmap,
    private val newBitmap: Bitmap,
    private val oldViewport: BoardViewport,
    private val newViewport: BoardViewport,
    private val oldTiles: List<List<Tile>>,
    private val newTiles: List<List<Tile>>
) : Animation {

    override fun hidesBoard(): Boolean = true

    override fun dirtyRects(): Array<Rect?> = arrayOf(unionBoardRect)

    private var stepIndex = 0

    private data class FlashTile(
        val rect: Rect,
        val completionS: Float,
        var phase: Int = 0
    )

    private fun isDone(): Boolean {
        // 1) Sweep must fully clear the union rect
        val sweepFinished = stepIndex * stepS > 2f + bandS

        // 2) Tiles may still need additional frames to complete flash phases
        val tilesFinished = flashTiles.all { it.phase >= TILE_FLASH_PHASES }

        return sweepFinished && tilesFinished
    }

    override fun drawOverEntities(canvas: Canvas) {
        if (isDone()) return

        drawFrame(canvas, stepIndex)
        stepIndex++

        val front = frontS
        val back = frontS - bandS

        for (tile in flashTiles) {
            if (tile.phase >= TILE_FLASH_PHASES) continue
            if (back < tile.completionS + gapS) continue

            if (tile.phase % 2 == 0) {
                canvas.withSave {
                    clipRect(tile.rect)
                    val paint = if (tile.phase == 0) flashBlackPaint else flashWhitePaint
                    canvas.drawRect(tile.rect, paint)
                }
            }

            tile.phase++
        }
    }

    override fun ticksUntilNextStep(): Int? =
        if (isDone()) null else TICKS_PER_STEP

    private val invertPaint = Paint().apply {
        colorFilter = ColorMatrixColorFilter(
            android.graphics.ColorMatrix(
                floatArrayOf(
                    -1f, 0f, 0f, 0f, 255f,
                    0f,-1f, 0f, 0f, 255f,
                    0f, 0f,-1f, 0f, 255f,
                    0f, 0f, 0f, 1f,   0f
                )
            )
        )
        isAntiAlias = false
    }

private val flashBlackPaint = Paint().apply {
    color = android.graphics.Color.BLACK
    isAntiAlias = false
}

private val flashWhitePaint = Paint().apply {
    color = android.graphics.Color.WHITE
    isAntiAlias = false
}

    private val oldBoardRect: Rect = Rect(
        (oldViewport.offsetX + oldViewport.cellSize).toInt(),
        (oldViewport.offsetY + oldViewport.cellSize).toInt(),
        (oldViewport.offsetX + (oldViewport.cols - 1) * oldViewport.cellSize).toInt(),
        (oldViewport.offsetY + (oldViewport.rows - 1) * oldViewport.cellSize).toInt()
    )

    private val newBoardRect: Rect = Rect(
        (newViewport.offsetX + newViewport.cellSize).toInt(),
        (newViewport.offsetY + newViewport.cellSize).toInt(),
        (newViewport.offsetX + (newViewport.cols - 1) * newViewport.cellSize).toInt(),
        (newViewport.offsetY + (newViewport.rows - 1) * newViewport.cellSize).toInt()
    )

    private val unionBoardRect: Rect = Rect().apply {
        set(oldBoardRect)
        union(newBoardRect)
    }

    private val W = unionBoardRect.width().toFloat()
    private val H = unionBoardRect.height().toFloat()
    private val L = unionBoardRect.left.toFloat()
    private val B = unionBoardRect.bottom.toFloat()

    // Step size expressed in sweep-space units (s ranges from 0..2).
    // STEP_FRACTION applies to rect width; height contribution follows aspect ratio.
    private val stepS = (STEP_PERCENT / 100f) * 2f
    private val bandS = 3f * stepS
    private val gapS = FLASH_GAP_STEPS * stepS
    private val frontS: Float
        get() = stepIndex * stepS

    private fun drawFrame(canvas: Canvas, stepIndex: Int) {
        val frontS = stepIndex * stepS

        // Draw the full previous frame first so areas outside unionBoardRect remain stable during the transition.
        canvas.drawBitmap(oldBitmap, 0f, 0f, null)

        val k0 = frontS - bandS
        if (k0 > -1000f) {
            // behind band: new bitmap normal
            drawSBand(canvas, -1000f, k0, newBitmap, null)
        }

        // live band thirds:
        // back third: new bitmap inverted for s in [frontS - bandS, frontS - 2*stepS]
        drawSBand(canvas, frontS - bandS, frontS - 2f * stepS, newBitmap, invertPaint)

        // middle third: new bitmap normal for s in [frontS - 2*stepS, frontS - stepS]
        drawSBand(canvas, frontS - 2f * stepS, frontS - stepS, newBitmap, null)

        // front third: old bitmap inverted for s in [frontS - stepS, frontS]
        drawSBand(canvas, frontS - stepS, frontS, oldBitmap, invertPaint)
    }

    private fun drawSBand(canvas: Canvas, a: Float, b: Float, bitmap: Bitmap, paint: Paint?) {
        val lo = min(a, b)
        val hi = maxOf(a, b)

        val left = unionBoardRect.left
        val right = unionBoardRect.right
        val topBound = unionBoardRect.top.toFloat()
        val bottomBound = unionBoardRect.bottom.toFloat()

        val sliceWidthPx = W * (STEP_PERCENT / 100f)
        var x = left.toFloat()
        while (x < right) {
            val x2 = min(x + sliceWidthPx, right.toFloat())

            // For fixed x, higher s => smaller y. So the top edge comes from the higher-s boundary (hi),
            // and the bottom edge comes from the lower-s boundary (lo).
            val yTop0 = yForS(hi, x)
            val yTop1 = yForS(hi, x2)
            val yBot0 = yForS(lo, x)
            val yBot1 = yForS(lo, x2)

            val top = min(yTop0, yTop1).coerceIn(topBound, bottomBound)
            val bottom = max(yBot0, yBot1).coerceIn(topBound, bottomBound)

            if (top < bottom) {
                val sliceRect = Rect(
                    x.toInt(),
                    top.toInt(),
                    x2.toInt(),
                    bottom.toInt()
                )
                canvas.withSave {
                    clipRect(unionBoardRect)
                    clipRect(sliceRect)
                    // Exclude stable wall regions from any band effect.
                    for (r in stableWallRects) {
                        canvas.clipOutRect(r)
                    }
                    canvas.drawBitmap(bitmap, 0f, 0f, paint)
                }
            }

            x = x2
        }
    }

    private fun yForS(k: Float, x: Float): Float {
        // s(x,y) = (x-L)/W + (B-y)/H
        // Solve for y: y = B - H * (k - (x-L)/W)
        return B - H * (k - (x - L) / W)
    }

    private fun sFor(x: Float, y: Float): Float {
        return (x - L) / W + (B - y) / H
    }

    // --- Region helpers and fields ---

    private fun interiorBoardRect(viewport: BoardViewport): Rect {
        val left = (viewport.offsetX + viewport.cellSize).toInt()
        val top = (viewport.offsetY + viewport.cellSize).toInt()
        val right = (viewport.offsetX + (viewport.cols - 1) * viewport.cellSize).toInt()
        val bottom = (viewport.offsetY + (viewport.rows - 1) * viewport.cellSize).toInt()
        return Rect(left, top, right, bottom)
    }

    private val oldInteriorRect by lazy { interiorBoardRect(oldViewport) }
    private val newInteriorRect by lazy { interiorBoardRect(newViewport) }

    // Compute a Region for walls given a viewport and tiles.
    private fun computeWallRegion(
        viewport: BoardViewport,
        tiles: List<List<Tile>>,
        interiorRect: Rect
    ): Region {
        val region = Region()

        // 1) Everything outside the interior is wall
        region.op(unionBoardRect, Region.Op.UNION)
        region.op(interiorRect, Region.Op.DIFFERENCE)

        // 2) Explicit wall tiles inside the interior
        val cell = viewport.cellSize.toInt()
        for (r in tiles.indices) {
            val row = tiles[r]
            for (c in row.indices) {
                if (row[c] == Tile.WALL) {
                    val left = (viewport.offsetX + (c + 1) * viewport.cellSize).toInt()
                    val top = (viewport.offsetY + (r + 1) * viewport.cellSize).toInt()
                    region.op(
                        Rect(left, top, left + cell, top + cell),
                        Region.Op.UNION
                    )
                }
            }
        }

        return region
    }

    // Compute a Region for floors/goals (not walls) in the new level, for the final flash.
    private fun computeNewFloorGoalRegion(viewport: BoardViewport, tiles: List<List<Tile>>): Region {
        val region = Region()
        val cell = viewport.cellSize.toInt()
        for (r in tiles.indices) {
            val row = tiles[r]
            for (c in row.indices) {
                val t = row[c]
                if (t == Tile.FLOOR || t == Tile.GOAL) {
                    val left = (viewport.offsetX + (c + 1) * viewport.cellSize).toInt()
                    val top = (viewport.offsetY + (r + 1) * viewport.cellSize).toInt()
                    region.op(Rect(left, top, left + cell, top + cell), Region.Op.UNION)
                }
            }
        }
        return region
    }

    // The region of stable walls (intersection of old and new wall regions).
    private val stableWallRegion: Region by lazy {
        val oldWalls = computeWallRegion(oldViewport, oldTiles, oldInteriorRect)
        val newWalls = computeWallRegion(newViewport, newTiles, newInteriorRect)
        oldWalls.apply { op(newWalls, Region.Op.INTERSECT) }
    }

    // List of rects covering stableWallRegion, for use in clipOutRect.
    private val stableWallRects: List<Rect> by lazy {
        val out = mutableListOf<Rect>()
        val it = RegionIterator(stableWallRegion)
        val r = Rect()
        while (it.next(r)) out.add(Rect(r))
        out
    }

    // Region of all new level floors/goals (excluding walls).
    private val newFloorGoalRegion: Region by lazy {
        computeNewFloorGoalRegion(newViewport, newTiles)
    }

    // Region of newFloorGoalRegion clipped to unionBoardRect.
    private val newFloorGoalRegionClipped: Region by lazy {
        val base = Region(unionBoardRect)
        base.op(newFloorGoalRegion, Region.Op.INTERSECT)
        base
    }

    private val flashTiles: List<FlashTile> by lazy {
        val out = mutableListOf<FlashTile>()
        val cell = newViewport.cellSize

        for (r in newTiles.indices) {
            val row = newTiles[r]
            for (c in row.indices) {
                val t = row[c]
                if (t == Tile.FLOOR || t == Tile.GOAL) {
                    val left = newViewport.offsetX + (c + 1) * cell
                    val top = newViewport.offsetY + (r + 1) * cell
                    val right = left + cell
                    val bottom = top + cell

                    val rect = Rect(
                        left.toInt(),
                        top.toInt(),
                        right.toInt(),
                        bottom.toInt()
                    )

                    val completionS = maxOf(
                        sFor(left, top),
                        sFor(right, top),
                        sFor(left, bottom),
                        sFor(right, bottom)
                    )

                    out.add(FlashTile(rect, completionS))
                }
            }
        }
        out
    }
}