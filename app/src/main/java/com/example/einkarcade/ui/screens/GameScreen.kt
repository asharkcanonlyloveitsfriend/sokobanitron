package com.example.einkarcade.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import android.os.Handler
import android.os.Looper
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.einkarcade.R
import com.example.einkarcade.GameController
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.*

@Composable
fun GameScreen(
    modifier: Modifier = Modifier,
    gameController: GameController,
    selectedBoxPosition: MutableState<Position?>
) {
    gameController.revision.value
    val playerPosition = gameController.playerPosition
    val syncError = remember { mutableStateOf<String?>(null) }
    val boxPainter = painterResource(id = R.drawable.box)
    val playerPainter = painterResource(id = R.drawable.player_slime)

    Box(modifier = modifier.fillMaxSize()) {
        fun handleTap(tappedPosition: Position) {
            val selectedBox = selectedBoxPosition.value

            if (gameController.boxPositions.contains(tappedPosition)) {
                if (selectedBox == tappedPosition) {
                    selectedBoxPosition.value = null
                } else {
                    selectedBoxPosition.value = tappedPosition
                }
            } else if (selectedBox != null) {
                selectedBoxPosition.value = null
                gameController.moveBoxTo(selectedBox, tappedPosition)
            } else {
                gameController.movePlayerTo(tappedPosition)
            }
        }

        Column(modifier = Modifier.fillMaxSize()) {
            val setExpanded = remember { mutableStateOf(false) }
            val setOptions = gameController.availableSetOptions

            Box(
                modifier = Modifier
                    .padding(16.dp)
                    .clickable { setExpanded.value = true }
            ) {
                Text("Set: ${gameController.currentSetName}", fontSize = 24.sp)
                DropdownMenu(
                    expanded = setExpanded.value,
                    onDismissRequest = { setExpanded.value = false }
                ) {
                    Column(
                        modifier = Modifier.heightIn(max = 800.dp)
                    ) {
                        setOptions.forEach { (id, name) ->
                            val isSelected = name == gameController.currentSetName
                            DropdownMenuItem(
                                text = { Text(name) },
                                onClick = {
                                    gameController.selectSetById(id)
                                    setExpanded.value = false
                                },
                                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .background(if (isSelected) Color.LightGray else Color.Transparent)
                            )
                        }
                    }
                }
            }

            val levelExpanded = remember { mutableStateOf(false) }
            val levels = gameController.levels()
            val currentLevelName = gameController.levelName
            val selectedLevelIndex = levels.indexOfFirst { it.name == currentLevelName }
            val levelScrollState = rememberScrollState()
            val density = LocalDensity.current
            val itemHeight: Dp = 40.dp

            Box(
                modifier = Modifier
                    .padding(start = 16.dp, bottom = 8.dp)
                    .clickable { levelExpanded.value = true }
            ) {
                Text("Level: ${gameController.levelName}", fontSize = 24.sp)
                DropdownMenu(
                    expanded = levelExpanded.value,
                    onDismissRequest = { levelExpanded.value = false }
                ) {
                    LaunchedEffect(levelExpanded.value, selectedLevelIndex) {
                        if (levelExpanded.value && selectedLevelIndex >= 0) {
                            val targetIndex = (selectedLevelIndex - 2).coerceAtLeast(0)
                            val targetOffset = with(density) { (itemHeight * targetIndex).roundToPx() }
                            levelScrollState.scrollTo(targetOffset)
                        }
                    }
                    Column(
                        modifier = Modifier
                            .heightIn(max = 800.dp)
                            .verticalScroll(levelScrollState)
                    ) {
                        levels.forEach { lvl ->
                            val completedMark = if (lvl.isCompleted) " ✓" else ""
                            val ratingBadge = when (lvl.rating) { 1 -> " 👍"; -1 -> " 👎"; else -> "" }
                            val isSelected = lvl.name == currentLevelName
                            DropdownMenuItem(
                                text = { Text(lvl.name + completedMark + ratingBadge) },
                                onClick = {
                                    gameController.selectLevel(lvl.name)
                                    levelExpanded.value = false
                                },
                                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .background(if (isSelected) Color.LightGray else Color.Transparent)
                            )
                        }
                    }
                }
            }

            Canvas(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .testTag("gameCanvas")
                    .pointerInput(Unit) {
                        detectTapGestures { offset ->
                            val col = ((offset.x - GRID_OFFSET_X) / CELL_SIZE).toInt()
                            val row = ((offset.y - GRID_OFFSET_Y) / CELL_SIZE).toInt()
                            if (!gameController.isGameWon &&
                                row in gameController.tiles.indices &&
                                col in gameController.tiles[0].indices
                            ) {
                                handleTap(Position(row, col))
                            }
                        }
                    }
            ) {
                for ((rowIndex, row) in gameController.tiles.withIndex()) {
                    for ((colIndex, tile) in row.withIndex()) {
                        when (tile) {
                            Tile.WALL -> drawWall(Position(rowIndex, colIndex))
                            Tile.GOAL -> drawGoal(Position(rowIndex, colIndex))
                            Tile.FLOOR -> drawFloor(Position(rowIndex, colIndex))
                            Tile.EMPTY -> {}
                        }
                    }
                }

                for (position in gameController.boxPositions) {
                    drawBox(position, boxPainter, position == selectedBoxPosition.value)
                }

                drawPlayer(playerPosition, playerPainter)
            }

            Row(
                modifier = Modifier.padding(16.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Button(
                    onClick = { gameController.restart() },
                    modifier = Modifier.focusProperties { canFocus = false }
                ) {
                    Text("Restart")
                }
                Button(
                    onClick = { gameController.undo() },
                    modifier = Modifier
                        .padding(start = 8.dp)
                        .focusProperties { canFocus = false }
                ) {
                    Text("Undo")
                }
                Button(
                    onClick = { gameController.previousLevel() },
                    modifier = Modifier
                        .padding(start = 8.dp)
                        .focusProperties { canFocus = false }
                ) {
                    Text("Previous")
                }
                Button(
                    onClick = { gameController.nextLevel() },
                    modifier = Modifier
                        .padding(start = 8.dp)
                        .focusProperties { canFocus = false }
                ) {
                    Text("Next")
                }
                // Rating buttons with a check when selected.
                val currentRating = gameController.getCurrentRating()

                Button(
                    onClick = { gameController.toggleThumbDown() },
                    modifier = Modifier
                        .padding(start = 8.dp)
                        .focusProperties { canFocus = false }
                ) {
                    val selected = currentRating == -1
                    Text(if (selected) "👎✓" else "👎")
                }
                Button(
                    onClick = { gameController.toggleThumbUp() },
                    modifier = Modifier
                        .padding(start = 8.dp)
                        .focusProperties { canFocus = false }
                ) {
                    val selected = currentRating == 1
                    Text(if (selected) "👍✓" else "👍")
                }
                Spacer(modifier = Modifier.weight(1f))
                Button(
                    onClick = {
                        syncError.value = null
                        val handler = Handler(Looper.getMainLooper())
                        Thread {
                            try {
                                gameController.syncWithServer()
                            } catch (t: Throwable) {
                                handler.post {
                                    syncError.value = "Sync failed."
                                }
                            }
                        }.start()
                    },
                    modifier = Modifier
                        .padding(start = 8.dp)
                        .focusProperties { canFocus = false }
                ) {
                    Text("Sync")
                }
            }
            if (syncError.value != null) {
                Text(
                    text = syncError.value ?: "",
                    color = Color.Black,
                    fontSize = 18.sp,
                    modifier = Modifier.padding(start = 16.dp, bottom = 8.dp)
                )
            }
        }

        if (gameController.isGameWon) {
            Box(
                modifier = Modifier
                    .fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Box(
                    modifier = Modifier
                        .padding(16.dp)
                        .background(Color.White)
                        .border(width = 2.dp, color = Color.Black)
                        .padding(16.dp)
                ) {
                    Text(
                        text = "You win!",
                        color = Color.Black,
                        fontSize = 32.sp
                    )
                }
            }
        }
    }
}
