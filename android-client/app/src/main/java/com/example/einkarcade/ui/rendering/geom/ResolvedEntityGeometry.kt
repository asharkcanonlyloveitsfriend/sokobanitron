package com.example.einkarcade.ui.rendering.geom

import android.graphics.Rect
import com.example.einkarcade.R
import com.example.einkarcade.ui.rendering.AndroidGameAssets

/**
 * Immutable, resolved geometry derived from the current BoardViewport.
 * Valid until the level layout or surface size changes.
 */
class ResolvedEntityGeometry(
    val boxBoundsPx: Rect,
    val boxSizePx: Int,
    val playerEyesOpaqueBoundsPx: Rect,
    val playerBoundsPx: Rect,
    val playerSizePx: Int,
    val playerInsetPx: Int,
) {
    companion object {
        private const val BOX_SCALE = 0.90f
        private const val PLAYER_SCALE = 0.80f
        private val playerEyesOpaqueBoundsCache = mutableMapOf<Int, Rect>()

        /**
         * Compute all resolved board geometry derived solely from tile size.
         * This must be called only when the tile size changes.
         */
        internal fun compute(
            tileSizePx: Float,
            assets: AndroidGameAssets,
        ): ResolvedEntityGeometry {
            val (boxSizePx, boxBoundsPx) = computeBoxGeometry(tileSizePx)
            val (playerSizePx, playerBoundsPx, playerInsetPx) =
                computePlayerGeometry(tileSizePx)

            return ResolvedEntityGeometry(
                boxBoundsPx = boxBoundsPx,
                boxSizePx = boxSizePx,
                playerEyesOpaqueBoundsPx =
                    computePlayerEyesOpaqueBounds(
                        sizePx = playerSizePx,
                        assets = assets,
                    ),
                playerBoundsPx = playerBoundsPx,
                playerSizePx = playerSizePx,
                playerInsetPx = playerInsetPx,
            )
        }

        private fun computeBoxGeometry(tileSizePx: Float): Pair<Int, Rect> {
            val boxSizePx =
                snapToWholePixel(tileSizePx * BOX_SCALE)
                    .toInt()
                    .coerceAtLeast(1)

            val boxInsetPx =
                snapToWholePixel((tileSizePx - boxSizePx) / 2f)
                    .toInt()

            val boxBoundsPx =
                Rect(
                    boxInsetPx,
                    boxInsetPx,
                    boxInsetPx + boxSizePx,
                    boxInsetPx + boxSizePx,
                )

            return boxSizePx to boxBoundsPx
        }

        private fun computePlayerGeometry(tileSizePx: Float): Triple<Int, Rect, Int> {
            val playerSizePx =
                snapToWholePixel(tileSizePx * PLAYER_SCALE)
                    .toInt()
                    .coerceAtLeast(1)

            val insetPx =
                snapToWholePixel(
                    (tileSizePx - playerSizePx) / 2f,
                ).toInt()

            val boundsPx =
                Rect(
                    insetPx,
                    insetPx,
                    insetPx + playerSizePx,
                    insetPx + playerSizePx,
                )

            return Triple(playerSizePx, boundsPx, insetPx)
        }

        /**
         * Compute the opaque bounds of the player's eyes, relative to the
         * tile origin (0,0), in pixel coordinates.
         */
        private fun computePlayerEyesOpaqueBounds(
            sizePx: Int,
            assets: AndroidGameAssets,
        ): Rect {
            val cached = playerEyesOpaqueBoundsCache[sizePx]
            if (cached != null) {
                return Rect(cached)
            }
            val computed =
                Rect(
                    assets.getOpaqueBounds(
                        R.drawable.player_eyes_blink,
                        sizePx,
                    ),
                )
            playerEyesOpaqueBoundsCache[sizePx] = Rect(computed)
            return computed
        }
    }
}
