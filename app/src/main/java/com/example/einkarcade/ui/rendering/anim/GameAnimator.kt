package com.example.einkarcade.ui.rendering.anim

import android.graphics.Rect
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.ui.rendering.AndroidGameAssets
import com.example.einkarcade.ui.rendering.RenderTimings
import com.example.einkarcade.ui.rendering.VanishSpec
import com.example.einkarcade.ui.rendering.geom.BoardViewport
import com.example.einkarcade.ui.rendering.geom.computeBoxPathDirtyRect
import com.example.einkarcade.ui.rendering.geom.computeVanishDirtyRect
import com.example.einkarcade.ui.rendering.geom.spriteDrawParams
import com.example.einkarcade.ui.rendering.model.RenderState

internal data class AnimationState(
    var boxPath: List<Position> = emptyList(),
    var boxPathActive: Boolean = false,
    var boxPathShrink: Float = 0f,
    var boxPathStartMs: Long = 0L,
    var boxPathDirtyRect: Rect? = null,
    var boxPathNeedsFinalClear: Boolean = false,
    var boxPathSuppressLine: Boolean = false,
    var playerSilhouettePosition: Position? = null,
    var playerSilhouetteStartMs: Long = 0L,
    var playerFlashPosition: Position? = null,
    var playerFlashStartMs: Long = 0L,
    var boxFlashPosition: Position? = null,
    var boxFlashStartMs: Long = 0L,
    var blinkStartMs: Long = 0L,
    var blinkEndMs: Long = 0L,
    var lastBlinkActive: Boolean = false,
    var vanishPosition: Position? = null,
    var vanishStartMs: Long = 0L,
    var vanishStep: Int? = null,
    var vanishLastPosition: Position? = null,
    var vanishNeedsFinalClear: Boolean = false
)

internal class GameAnimator(private val assets: AndroidGameAssets) {
    val state = AnimationState()

    fun reset() {
        state.boxPath = emptyList()
        state.boxPathActive = false
        state.boxPathShrink = 0f
        state.boxPathStartMs = 0L
        state.boxPathDirtyRect = null
        state.boxPathNeedsFinalClear = false
        state.boxPathSuppressLine = false
        state.playerSilhouettePosition = null
        state.playerSilhouetteStartMs = 0L
        state.playerFlashPosition = null
        state.playerFlashStartMs = 0L
        state.boxFlashPosition = null
        state.boxFlashStartMs = 0L
        state.blinkStartMs = 0L
        state.blinkEndMs = 0L
        state.lastBlinkActive = false
        state.vanishPosition = null
        state.vanishStartMs = 0L
        state.vanishStep = null
        state.vanishLastPosition = null
        state.vanishNeedsFinalClear = false
    }

    fun startBoxPath(
        path: List<Position>,
        pendingPlayer: Position,
        displayedPlayer: Position,
        viewport: BoardViewport?,
        suppressLine: Boolean,
        nowMs: Long,
        renderState: RenderState
    ) {
        require(path.size >= 2) { "Box path requires at least two points." }
        state.boxPath = path
        renderState.pendingPlayerPosition = pendingPlayer
        state.boxPathStartMs = nowMs
        state.boxPathShrink = 0f
        state.boxPathActive = true
        state.boxPathNeedsFinalClear = false
        state.boxPathSuppressLine = suppressLine

        state.boxPathDirtyRect = viewport?.let {
            computeBoxPathDirtyRect(it, path, displayedPlayer, pendingPlayer)
        }
    }

    fun startVanish(at: Position, nowMs: Long) {
        state.vanishPosition = at
        state.vanishStartMs = nowMs
        state.vanishStep = 0
        state.vanishLastPosition = at
        state.vanishNeedsFinalClear = false
    }

    fun startPlayerFlash(from: Position?, nowMs: Long) {
        state.playerFlashPosition = from
        state.playerFlashStartMs = nowMs
    }

    fun startBoxFlash(from: Position, nowMs: Long) {
        state.boxFlashPosition = from
        state.boxFlashStartMs = nowMs
    }

    fun startPlayerSilhouette(position: Position?, nowMs: Long) {
        state.playerSilhouettePosition = position
        state.playerSilhouetteStartMs = nowMs
    }

    fun triggerBlink(nowMs: Long, delayMs: Long = RenderTimings.BLINK_DELAY_MS) {
        val start = nowMs + delayMs
        state.blinkStartMs = start
        state.blinkEndMs = start + RenderTimings.BLINK_DURATION_MS
        state.lastBlinkActive = false
    }

    fun isBlinking(nowMs: Long): Boolean {
        return nowMs in state.blinkStartMs until state.blinkEndMs
    }

    fun tick(nowMs: Long, viewport: BoardViewport?, renderState: RenderState): TickResult {
        val changed = updateBoxPathAnimation(nowMs, renderState)
        val vanishChanged = updateVanishAnimation(nowMs)
        val blinkActive = isBlinking(nowMs)
        val pendingBlink = !blinkActive && state.blinkStartMs > nowMs
        val playerFlashActive =
            state.playerFlashPosition != null &&
                (nowMs - state.playerFlashStartMs) <= RenderTimings.FLASH_DURATION_MS
        val boxFlashActive =
            state.boxFlashPosition != null &&
                (nowMs - state.boxFlashStartMs) <= RenderTimings.FLASH_DURATION_MS

        var dirtyRect: Rect? = null
        var requestedRender = false

        if (blinkActive != state.lastBlinkActive) {
            state.lastBlinkActive = blinkActive
            if (!changed && !vanishChanged) {
                requestedRender = true
                dirtyRect = if (state.boxPathActive || state.boxPathNeedsFinalClear) {
                    union(dirtyRect, computeBoxPathDirtyUnion(nowMs, viewport))
                } else {
                    union(dirtyRect, computeBlinkDirtyRect(viewport, renderState))
                }
            }
        }

        if (changed) {
            requestedRender = true
            dirtyRect = union(dirtyRect, computeBoxPathDirtyUnion(nowMs, viewport))
        }

        if (vanishChanged && !changed) {
            requestedRender = true
            dirtyRect = if (state.boxPathActive || state.boxPathNeedsFinalClear) {
                union(dirtyRect, computeBoxPathDirtyUnion(nowMs, viewport))
            } else {
                union(dirtyRect, computeVanishDirtyRect(viewport))
            }
        }

        if (!changed && !vanishChanged && !blinkActive && playerFlashActive && !state.boxPathActive) {
            requestedRender = true
            dirtyRect = union(dirtyRect, computePlayerFlashDirtyRect(viewport))
        }

        if (!changed && !vanishChanged && !blinkActive && boxFlashActive && !state.boxPathActive) {
            requestedRender = true
            val position = state.boxFlashPosition
            if (position != null && viewport != null) {
                dirtyRect = union(dirtyRect, spriteDrawParams(viewport, position, 0.90f).dirtyRect)
            }
        }

        if (!playerFlashActive && state.playerFlashPosition != null && !state.boxPathActive) {
            requestedRender = true
            val clearedPos = state.playerFlashPosition
            state.playerFlashPosition = null
            state.playerFlashStartMs = 0L
            if (clearedPos != null && viewport != null) {
                dirtyRect = union(dirtyRect, spriteDrawParams(viewport, clearedPos, 0.80f).dirtyRect)
            }
        }

        if (!boxFlashActive && state.boxFlashPosition != null && !state.boxPathActive) {
            requestedRender = true
            val clearedPos = state.boxFlashPosition
            state.boxFlashPosition = null
            state.boxFlashStartMs = 0L
            if (clearedPos != null && viewport != null) {
                dirtyRect = union(dirtyRect, spriteDrawParams(viewport, clearedPos, 0.90f).dirtyRect)
            }
        }

        if (state.vanishNeedsFinalClear && state.vanishPosition == null) {
            requestedRender = true
            dirtyRect = union(dirtyRect, computeVanishDirtyRect(viewport))
            state.vanishNeedsFinalClear = false
            state.vanishLastPosition = null
        }

        if (state.boxPathNeedsFinalClear && !state.boxPathActive) {
            state.boxPathDirtyRect = null
            state.boxPathNeedsFinalClear = false
        }

        val vanishActive = state.vanishPosition != null
        var needsNextFrame =
            state.boxPathActive || blinkActive || vanishActive || playerFlashActive || boxFlashActive
        var nextFrameDelayMs: Long? = null

        if (!needsNextFrame && pendingBlink) {
            needsNextFrame = true
            nextFrameDelayMs = (state.blinkStartMs - nowMs).coerceAtLeast(0L)
        }

        val forceFullRender = requestedRender && dirtyRect == null

        return TickResult(
            dirtyRect = dirtyRect,
            needsNextFrame = needsNextFrame,
            forceFullRender = forceFullRender,
            nextFrameDelayMs = nextFrameDelayMs
        )
    }

    fun computeBoxPathDirtyUnion(
        nowMs: Long,
        viewport: BoardViewport?
    ): Rect? {
        return computeBoxPathDirtyUnionInternal(nowMs, viewport)
    }

    private fun updateBoxPathAnimation(nowMs: Long, renderState: RenderState): Boolean {
        if (!state.boxPathActive) return false
        val elapsed = nowMs - state.boxPathStartMs
        if (elapsed < RenderTimings.BOX_PATH_DELAY_MS) return false
        val progress = if (state.boxPathSuppressLine) {
            1f
        } else {
            ((elapsed - RenderTimings.BOX_PATH_DELAY_MS).toFloat() /
                RenderTimings.BOX_PATH_DURATION_MS.toFloat()).coerceAtMost(1f)
        }
        var changed = false
        if (progress != state.boxPathShrink) {
            state.boxPathShrink = progress
            changed = true
        }
        if (elapsed >= RenderTimings.BOX_PATH_DELAY_MS +
            if (state.boxPathSuppressLine) 0L else RenderTimings.BOX_PATH_DURATION_MS) {
            state.boxPathActive = false
            state.boxPathNeedsFinalClear = true
            state.boxPathSuppressLine = false
            val pending = renderState.pendingPlayerPosition
            if (pending != null) {
                renderState.displayedPlayerPosition = pending
                renderState.pendingPlayerPosition = null
            }
            renderState.pendingFacingLeft?.let { facing ->
                renderState.isFacingLeft = facing
            }
            renderState.pendingFacingLeft = null
            changed = true
        }
        return changed
    }

    private fun updateVanishAnimation(nowMs: Long): Boolean {
        val currentPosition = state.vanishPosition ?: return false
        val elapsed = nowMs - state.vanishStartMs
        var cumulative = 0L

        for (step in 0..VanishSpec.LAST_STEP) {
            val delay = VanishSpec.delayMs(step)
            if (elapsed < cumulative + delay) {
                if (state.vanishStep != step) {
                    state.vanishStep = step
                    return true
                }
                return false
            }
            cumulative += delay
        }

        if (state.vanishStep != null) {
            state.vanishStep = null
            state.vanishPosition = null
            state.vanishLastPosition = currentPosition
            state.vanishNeedsFinalClear = true
            triggerBlink(nowMs, delayMs = 0L)
            return true
        }

        return false
    }

    private fun computeBoxPathDirtyUnionInternal(
        nowMs: Long,
        viewport: BoardViewport?
    ): Rect? {
        if (viewport == null) return null
        val boxDirty = state.boxPathDirtyRect
        val vanishTarget = if (state.vanishPosition != null || state.vanishNeedsFinalClear) {
            state.vanishPosition ?: state.vanishLastPosition
        } else {
            null
        }
        val vanishDirty = vanishTarget?.let { computeVanishDirtyRect(viewport, it) }
        var dirty = when {
            boxDirty != null && vanishDirty != null -> Rect(boxDirty).apply { union(vanishDirty) }
            boxDirty != null -> Rect(boxDirty)
            vanishDirty != null -> Rect(vanishDirty)
            else -> null
        }
        if (state.boxFlashPosition != null &&
            nowMs - state.boxFlashStartMs <= RenderTimings.FLASH_DURATION_MS) {
            val flashRect = spriteDrawParams(viewport, state.boxFlashPosition!!, 0.90f).dirtyRect
            dirty = union(dirty, flashRect)
        }
        val silhouettePos = state.playerSilhouettePosition
        if (silhouettePos != null) {
            val silhouetteRect = spriteDrawParams(viewport, silhouettePos, 0.80f).dirtyRect
            dirty = union(dirty, silhouetteRect)
        }
        val playerFlashPos = state.playerFlashPosition
        if (playerFlashPos != null &&
            nowMs - state.playerFlashStartMs <= RenderTimings.FLASH_DURATION_MS) {
            val flashRect = spriteDrawParams(viewport, playerFlashPos, 0.80f).dirtyRect
            dirty = union(dirty, flashRect)
        }
        return dirty
    }

    private fun computeVanishDirtyRect(viewport: BoardViewport?): Rect? {
        if (viewport == null) return null
        val position = state.vanishPosition ?: state.vanishLastPosition ?: return null
        return computeVanishDirtyRect(viewport, position)
    }

    private fun computePlayerFlashDirtyRect(viewport: BoardViewport?): Rect? {
        if (viewport == null) return null
        val position = state.playerFlashPosition ?: return null
        return spriteDrawParams(viewport, position, 0.80f).dirtyRect
    }

    private fun computeBlinkDirtyRect(viewport: BoardViewport?, renderState: RenderState): Rect? {
        if (viewport == null) return null
        val playerPos = renderState.playerPosition ?: return null
        val params = spriteDrawParams(viewport, playerPos, 0.80f)
        val openBounds = assets.getOpaqueBounds(R.drawable.player_eyes_open, params.sizePx)
        val blinkBounds = assets.getOpaqueBounds(R.drawable.player_eyes_blink, params.sizePx)
        val bounds = Rect(openBounds)
        bounds.union(blinkBounds)
        if (bounds.isEmpty) {
            error("Dirty render requested with empty blink bounds.")
        }
        val paddingPx = 2
        val left = params.left.toInt() + bounds.left - paddingPx
        val top = params.top.toInt() + bounds.top - paddingPx
        val right = params.left.toInt() + bounds.right + paddingPx
        val bottom = params.top.toInt() + bounds.bottom + paddingPx
        return Rect(left, top, right, bottom)
    }

    private fun union(base: Rect?, extra: Rect?): Rect? {
        if (extra == null) return base
        return if (base == null) {
            Rect(extra)
        } else {
            base.apply { union(extra) }
        }
    }
}
