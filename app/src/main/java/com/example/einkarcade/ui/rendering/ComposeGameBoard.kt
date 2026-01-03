package com.example.einkarcade.ui.rendering

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.painter.Painter
import androidx.compose.ui.input.pointer.pointerInput
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile

internal data class ComposeGameAssets(
    val boxPainter: Painter,
    val selectedBoxPainter: Painter,
    val playerPainter: Painter,
    val openEyesPainter: Painter,
    val blinkEyesPainter: Painter
)

@Composable
internal fun ComposeGameBoard(
    scene: GameScene,
    assets: ComposeGameAssets,
    isGameWon: Boolean,
    modifier: Modifier = Modifier,
    onTapCell: (Position) -> Unit
) {
    Canvas(
        modifier = modifier.pointerInput(scene, isGameWon) {
            detectTapGestures { offset ->
                if (isGameWon) return@detectTapGestures
                val viewport = computeBoardViewport(
                    surfaceWidth = size.width.toFloat(),
                    surfaceHeight = size.height.toFloat(),
                    innerRows = scene.tiles.size,
                    innerCols = scene.tiles.first().size
                )
                val tappedPosition = viewport.screenToInnerCell(offset.x, offset.y)
                if (tappedPosition != null) {
                    onTapCell(tappedPosition)
                }
            }
        }
    ) {
        val viewport = computeBoardViewport(
            surfaceWidth = size.width,
            surfaceHeight = size.height,
            innerRows = scene.tiles.size,
            innerCols = scene.tiles.first().size
        )
        val cellSize = viewport.cellSize
        val offsetX = viewport.offsetX
        val offsetY = viewport.offsetY

        for ((rowIndex, row) in scene.tiles.withIndex()) {
            for ((colIndex, tile) in row.withIndex()) {
                val paddedRow = rowIndex + 1
                val paddedCol = colIndex + 1
                when (tile) {
                    Tile.GOAL -> drawGoal(Position(paddedRow, paddedCol), cellSize, offsetX, offsetY)
                    Tile.FLOOR -> drawFloor(Position(paddedRow, paddedCol), cellSize, offsetX, offsetY)
                    Tile.WALL -> {
                        drawVanishingBox(
                            vanish = scene.vanish,
                            gridPosition = Position(rowIndex, colIndex),
                            paddedPosition = Position(paddedRow, paddedCol),
                            boxPainter = assets.boxPainter,
                            selectedBoxPainter = assets.selectedBoxPainter,
                            cellSize = cellSize,
                            offsetX = offsetX,
                            offsetY = offsetY
                        )
                    }
                }
            }
        }

        drawBoxPathLine(
            isActive = scene.boxPathActive,
            shrink = scene.boxPathShrink,
            path = scene.boxPath,
            cellSize = cellSize,
            offsetX = offsetX,
            offsetY = offsetY
        )

        for (position in scene.boxPositions) {
            drawBox(
                Position(position.row + 1, position.col + 1),
                assets.boxPainter,
                assets.selectedBoxPainter,
                position == scene.selectedBox,
                cellSize,
                offsetX,
                offsetY
            )
        }

        val drawnPlayerPosition = scene.playerPosition
        val flipPlayer = scene.isFacingLeft
        drawPlayer(
            Position(drawnPlayerPosition.row + 1, drawnPlayerPosition.col + 1),
            assets.playerPainter,
            flipPlayer,
            cellSize,
            offsetX,
            offsetY
        )
        drawPlayer(
            Position(drawnPlayerPosition.row + 1, drawnPlayerPosition.col + 1),
            if (scene.isBlinking) assets.blinkEyesPainter else assets.openEyesPainter,
            flipPlayer,
            cellSize,
            offsetX,
            offsetY
        )
    }
}
