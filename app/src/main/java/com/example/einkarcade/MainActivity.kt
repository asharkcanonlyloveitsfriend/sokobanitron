package com.example.einkarcade

import android.os.Bundle
import android.util.Log
import android.view.KeyEvent
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import com.example.einkarcade.ui.theme.EinkArcadeTheme

enum class Direction { UP, DOWN, LEFT, RIGHT }
enum class Tile { EMPTY }

data class Position(val row: Int, val col: Int)
data class GameState(
    val grid: List<List<Tile>>
) {
    fun isInBounds(position: Position): Boolean {
        return position.row in grid.indices && position.col in grid[0].indices
    }
}

class MainActivity : ComponentActivity() {

    companion object {
        internal const val CELL_SIZE = 100f
        internal const val GAME_LEFT = 50f
        internal const val GAME_TOP = 0f
        internal const val GRID_WIDTH = 6
        internal const val GRID_HEIGHT = 6
        internal const val GAME_WIDTH = CELL_SIZE * GRID_WIDTH
        internal const val GAME_HEIGHT = CELL_SIZE * GRID_HEIGHT
        internal val PLAYER_POSITION = mutableStateOf(Position(0, 0))
        internal val GAME_STATE = GameState(
            grid = List(GRID_HEIGHT) { row ->
                List(GRID_WIDTH) { col ->
                    Tile.EMPTY
                }
            }
        )
    }

    private fun move(direction: Direction) {
        Log.d("GameInput", "Move: $direction")

        val (playerRow, playerCol) = PLAYER_POSITION.value

        val newPosition = when (direction) {
            Direction.UP -> Position(playerRow - 1, playerCol)
            Direction.DOWN -> Position(playerRow + 1, playerCol)
            Direction.LEFT -> Position(playerRow, playerCol - 1)
            Direction.RIGHT -> Position(playerRow, playerCol + 1)
        }

        if (!GAME_STATE.isInBounds(newPosition)) return
        PLAYER_POSITION.value = newPosition
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            EinkArcadeTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    GameScreen(
                        modifier = Modifier.padding(innerPadding)
                    )
                }
            }
        }
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent?): Boolean {
        when (keyCode) {
            KeyEvent.KEYCODE_DPAD_DOWN -> move(Direction.DOWN)
            KeyEvent.KEYCODE_DPAD_UP -> move(Direction.UP)
            KeyEvent.KEYCODE_DPAD_LEFT -> move(Direction.LEFT)
            KeyEvent.KEYCODE_DPAD_RIGHT -> move(Direction.RIGHT)
            else -> {
                Log.d("GameInput", "KeyDown: $keyCode")
            }
        }
        return true
    }
}

@Composable
fun GameScreen(modifier: Modifier = Modifier) {
    Canvas(modifier = modifier.fillMaxSize()) {
        drawRect(
            color = androidx.compose.ui.graphics.Color.DarkGray,
            topLeft = androidx.compose.ui.geometry.Offset(MainActivity.GAME_LEFT, MainActivity.GAME_TOP),
            size = androidx.compose.ui.geometry.Size(MainActivity.GAME_WIDTH, MainActivity.GAME_HEIGHT),
            style = androidx.compose.ui.graphics.drawscope.Stroke(width = 4f)
        )
        val player = MainActivity.PLAYER_POSITION.value
        for (row in 0 until MainActivity.GRID_HEIGHT) {
            for (col in 0 until MainActivity.GRID_WIDTH) {
                val x = MainActivity.GAME_LEFT + col * MainActivity.CELL_SIZE
                val y = MainActivity.GAME_TOP + row * MainActivity.CELL_SIZE
                drawRect(
                    color = androidx.compose.ui.graphics.Color.LightGray,
                    topLeft = androidx.compose.ui.geometry.Offset(x, y),
                    size = androidx.compose.ui.geometry.Size(MainActivity.CELL_SIZE, MainActivity.CELL_SIZE),
                    style = androidx.compose.ui.graphics.drawscope.Stroke(width = 1f)
                )
                if (row == player.row && col == player.col) {
                    drawRect(
                        color = androidx.compose.ui.graphics.Color.Gray,
                        topLeft = androidx.compose.ui.geometry.Offset(x, y),
                        size = androidx.compose.ui.geometry.Size(MainActivity.CELL_SIZE, MainActivity.CELL_SIZE)
                    )
                }
            }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun GameScreenPreview() {
    EinkArcadeTheme {
        GameScreen()
    }
}