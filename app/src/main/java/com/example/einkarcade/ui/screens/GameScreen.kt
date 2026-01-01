package com.example.einkarcade.ui.screens

import kotlin.math.min
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
import androidx.compose.foundation.Image
import androidx.compose.ui.layout.ContentScale
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
        Image(
            painter = painterResource(id = R.drawable.bg_space),
            contentDescription = null,
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Crop
        )
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
                Text(
                    text = gameController.currentSetName,
                    fontSize = 16.sp,
                    color = Color.LightGray,
                    modifier = Modifier
                        .background(Color.Black, shape = androidx.compose.foundation.shape.RoundedCornerShape(6.dp))
                        .padding(horizontal = 6.dp, vertical = 2.dp)
                )
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

            Spacer(modifier = Modifier.padding(bottom = 2.dp))

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
                Text(
                    text = gameController.levelName,
                    fontSize = 16.sp,
                    color = Color.LightGray,
                    modifier = Modifier
                        .background(Color.Black, shape = androidx.compose.foundation.shape.RoundedCornerShape(6.dp))
                        .padding(horizontal = 6.dp, vertical = 2.dp)
                )
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
                            val rows = gameController.tiles.size + 2
                            val cols = (gameController.tiles.firstOrNull()?.size ?: 0) + 2
                            if (rows == 0 || cols == 0) return@detectTapGestures

                            val tileSizeByWidth = size.width / cols
                            val tileSizeByHeight = size.height / rows
                            val cellSize = min(tileSizeByWidth, tileSizeByHeight)

                            val renderedWidth = cellSize * cols
                            val renderedHeight = cellSize * rows

                            val offsetX = (size.width - renderedWidth) / 2f
                            val offsetY = (size.height - renderedHeight) / 2f

                            val col = ((offset.x - offsetX) / cellSize).toInt()
                            val row = ((offset.y - offsetY) / cellSize).toInt()
                            val innerRow = row - 1
                            val innerCol = col - 1
                            if (!gameController.isGameWon &&
                                innerRow in gameController.tiles.indices &&
                                innerCol in gameController.tiles[0].indices
                            ) {
                                handleTap(Position(innerRow, innerCol))
                            }
                        }
                    }
            ) {
                val rows = gameController.tiles.size + 2
                val cols = (gameController.tiles.firstOrNull()?.size ?: 0) + 2

                if (rows == 0 || cols == 0) return@Canvas

                val tileSizeByWidth = size.width / cols
                val tileSizeByHeight = size.height / rows
                val cellSize = min(tileSizeByWidth, tileSizeByHeight)

                val renderedWidth = cellSize * cols
                val renderedHeight = cellSize * rows

                val offsetX = (size.width - renderedWidth) / 2f
                val offsetY = (size.height - renderedHeight) / 2f

                for ((rowIndex, row) in gameController.tiles.withIndex()) {
                    for ((colIndex, tile) in row.withIndex()) {
                        val paddedRow = rowIndex + 1
                        val paddedCol = colIndex + 1
                        when (tile) {
                            Tile.GOAL -> drawGoal(Position(paddedRow, paddedCol), cellSize, offsetX, offsetY)
                            Tile.FLOOR -> drawFloor(Position(paddedRow, paddedCol), cellSize, offsetX, offsetY)
                            Tile.WALL -> {}
                        }
                    }
                }

                for (position in gameController.boxPositions) {
                    drawBox(Position(position.row + 1, position.col + 1), boxPainter, position == selectedBoxPosition.value, cellSize, offsetX, offsetY)
                }

                drawPlayer(Position(playerPosition.row + 1, playerPosition.col + 1), playerPainter, cellSize, offsetX, offsetY)
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
