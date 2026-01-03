package com.example.einkarcade.ui.rendering

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView
import com.example.einkarcade.sokoban.Position

@Composable
internal fun SurfaceGameBoard(
    scene: GameScene,
    isGameWon: Boolean,
    modifier: Modifier = Modifier,
    onTapCell: (Position) -> Unit
) {
    AndroidView(
        modifier = modifier,
        factory = { context -> GameSurfaceView(context) },
        update = { view ->
            view.setContent(scene, isGameWon, onTapCell)
        }
    )
}
