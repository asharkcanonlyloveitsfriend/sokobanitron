package com.example.einkarcade.ui.screens

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Android
import androidx.compose.material.icons.filled.ChevronLeft
import androidx.compose.material.icons.filled.ChevronRight
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material.icons.filled._360
import androidx.compose.material.icons.outlined.FavoriteBorder
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.vectorResource
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.example.einkarcade.GameController
import com.example.einkarcade.R
import com.example.einkarcade.sokoban.Position
import com.example.einkarcade.sokoban.Tile
import com.example.einkarcade.ui.rendering.*
import kotlin.math.min


@Composable
fun GameScreen(
    modifier: Modifier = Modifier,
    gameController: GameController,
    selectedBoxPosition: MutableState<Position?>
) {
    gameController.revision.value
    val playerPosition = gameController.playerPosition
    val syncError = remember { mutableStateOf<String?>(null) }
    val syncSuccess = remember { mutableStateOf(false) }
    val backDownTime = remember { mutableStateOf<Long?>(null) }
    val boxPainter = painterResource(id = R.drawable.box)
    val playerPainter = painterResource(id = R.drawable.player_slime)
    val focusRequester = remember { FocusRequester() }
    data class VanishState(val position: Position, val step: Int)
    val vanishingTile = remember { mutableStateOf<VanishState?>(null) }

    BackHandler(enabled = true) {
        // handled manually via key events below
    }

    LaunchedEffect(Unit) {
        focusRequester.requestFocus()
    }

    Box(
        modifier = modifier
            .fillMaxSize()
            .focusRequester(focusRequester)
            .focusable()
            .onKeyEvent { event ->
                if (event.key == Key.Back) {
                    when (event.type) {
                        KeyEventType.KeyDown -> {
                            if (backDownTime.value == null) {
                                backDownTime.value = SystemClock.elapsedRealtime()
                            }
                            true
                        }
                        KeyEventType.KeyUp -> {
                            val downTime = backDownTime.value
                            backDownTime.value = null
                            if (downTime != null) {
                                val duration = SystemClock.elapsedRealtime() - downTime
                                if (duration >= 500) {
                                    // Long press → Restart
                                    gameController.restart()
                                } else {
                                    // Short press → Undo
                                    gameController.undo()
                                }
                            }
                            true
                        }
                        else -> false
                    }
                } else {
                    false
                }
            }
    ) {
        Image(
            painter = painterResource(id = R.drawable.bg_space),
            contentDescription = null,
            modifier = Modifier.fillMaxSize(),
            contentScale = ContentScale.Crop
        )
        fun advanceVanish(step: Int, position: Position) {
            if (step >= 4) {
                vanishingTile.value = null
                return
            }

            vanishingTile.value = VanishState(position, step + 1)

            Handler(Looper.getMainLooper()).postDelayed(
                { advanceVanish(step + 1, position) },
                100
            )
        }

        fun handleTap(tappedPosition: Position) {
            val tile = gameController.tiles[tappedPosition.row][tappedPosition.col]
            if (tile == Tile.WALL) {
                vanishingTile.value = VanishState(tappedPosition, 0)
                advanceVanish(0, tappedPosition)
                return
            }
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
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                // --- Set (top-left) ---
                val setExpanded = remember { mutableStateOf(false) }
                val setOptions = gameController.availableSetOptions

                Box(
                    modifier = Modifier
                        .clickable { setExpanded.value = true }
                ) {
                    Text(
                        text = gameController.currentSetName,
                        fontSize = 16.sp,
                        color = Color.LightGray,
                        modifier = Modifier
                            .background(
                                Color.Black,
                                shape = androidx.compose.foundation.shape.RoundedCornerShape(6.dp)
                            )
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
                                        .background(
                                            if (isSelected) Color.LightGray else Color.Transparent
                                        )
                                )
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.weight(1f))

                // --- Level (top-right, right-aligned) ---
                val levelExpanded = remember { mutableStateOf(false) }
                val levels = gameController.levels()
                val currentLevelName = gameController.levelName
                val selectedLevelIndex = levels.indexOfFirst { it.name == currentLevelName }
                val levelScrollState = rememberScrollState()
                val density = LocalDensity.current
                val itemHeight: Dp = 40.dp

                Box(
                    modifier = Modifier
                        .clickable { levelExpanded.value = true }
                ) {
                    Text(
                        text = gameController.levelName,
                        fontSize = 16.sp,
                        color = Color.LightGray,
                        modifier = Modifier
                            .background(
                                Color.Black,
                                shape = androidx.compose.foundation.shape.RoundedCornerShape(6.dp)
                            )
                            .padding(horizontal = 6.dp, vertical = 2.dp)
                    )

                    DropdownMenu(
                        expanded = levelExpanded.value,
                        onDismissRequest = { levelExpanded.value = false }
                    ) {
                        LaunchedEffect(levelExpanded.value, selectedLevelIndex) {
                            if (levelExpanded.value && selectedLevelIndex >= 0) {
                                val targetIndex = (selectedLevelIndex - 2).coerceAtLeast(0)
                                val targetOffset =
                                    with(density) { (itemHeight * targetIndex).roundToPx() }
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
                                val ratingBadge =
                                    when (lvl.rating) { 1 -> " 👍"; -1 -> " 👎"; else -> "" }
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
                                        .background(
                                            if (isSelected) Color.LightGray else Color.Transparent
                                        )
                                )
                            }
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
                            Tile.WALL -> {
                                val vanish = vanishingTile.value
                                if (vanish != null && vanish.position == Position(rowIndex, colIndex)) {
                                    when (vanish.step) {
                                        0 -> {
                                            // normal-size dark box
                                            drawRect(
                                                color = Color.DarkGray,
                                                topLeft = androidx.compose.ui.geometry.Offset(
                                                    offsetX + paddedCol * cellSize + cellSize * 0.1f,
                                                    offsetY + paddedRow * cellSize + cellSize * 0.1f
                                                ),
                                                size = androidx.compose.ui.geometry.Size(
                                                    cellSize * 0.8f,
                                                    cellSize * 0.8f
                                                )
                                            )
                                        }
                                        1 -> {
                                            drawRect(
                                                color = Color.DarkGray,
                                                topLeft = androidx.compose.ui.geometry.Offset(
                                                    offsetX + paddedCol * cellSize + cellSize * 0.2f,
                                                    offsetY + paddedRow * cellSize + cellSize * 0.2f
                                                ),
                                                size = androidx.compose.ui.geometry.Size(
                                                    cellSize * 0.6f,
                                                    cellSize * 0.6f
                                                )
                                            )
                                        }
                                        2 -> {
                                            drawRect(
                                                color = Color.Black,
                                                topLeft = androidx.compose.ui.geometry.Offset(
                                                    offsetX + paddedCol * cellSize + cellSize * 0.3f,
                                                    offsetY + paddedRow * cellSize + cellSize * 0.3f
                                                ),
                                                size = androidx.compose.ui.geometry.Size(
                                                    cellSize * 0.4f,
                                                    cellSize * 0.4f
                                                )
                                            )
                                        }
                                        3 -> {
                                            // full-tile white flash
                                            drawRect(
                                                color = Color.White,
                                                topLeft = androidx.compose.ui.geometry.Offset(
                                                    offsetX + paddedCol * cellSize,
                                                    offsetY + paddedRow * cellSize
                                                ),
                                                size = androidx.compose.ui.geometry.Size(cellSize, cellSize)
                                            )
                                        }
                                    }
                                } else {
                                    // no-op: wall is invisible, background shows through
                                }
                            }
                        }
                    }
                }

                for (position in gameController.boxPositions) {
                    drawBox(Position(position.row + 1, position.col + 1), boxPainter, position == selectedBoxPosition.value, cellSize, offsetX, offsetY)
                }

                drawPlayer(Position(playerPosition.row + 1, playerPosition.col + 1), playerPainter, cellSize, offsetX, offsetY)
            }

            @Composable
            fun BottomIconButton(
                onClick: () -> Unit,
                icon: ImageVector,
                contentDescription: String
            ) {
                val interactionSource = remember { MutableInteractionSource() }
                val isPressed = interactionSource.collectIsPressedAsState()

                Box(
                    modifier = Modifier
                        .height(48.dp)
                        .background(if (isPressed.value) Color.DarkGray else Color.Black)
                        .clickable(
                            interactionSource = interactionSource,
                            indication = null,
                            onClick = onClick
                        )
                        .padding(horizontal = 12.dp)
                        .focusProperties { canFocus = false },
                    contentAlignment = Alignment.Center
                ) {
                    Icon(
                        imageVector = icon,
                        contentDescription = contentDescription,
                        tint = Color.LightGray
                    )
                }
            }

            Row(
                modifier = Modifier.padding(16.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                BottomIconButton(
                    onClick = { gameController.previousLevel() },
                    icon = Icons.Filled.ChevronLeft,
                    contentDescription = "Previous level"
                )

                BottomIconButton(
                    onClick = { gameController.nextLevel() },
                    icon = Icons.Filled.ChevronRight,
                    contentDescription = "Next level"
                )
                val currentRating = gameController.getCurrentRating()

                // --- X (dislike) ---
                BottomIconButton(
                    onClick = {
                        syncSuccess.value = false
                        syncError.value = null
                        gameController.toggleThumbDown()
                    },
                    icon = ImageVector.vectorResource(
                        if (currentRating == -1) R.drawable.ic_dislike_filled else R.drawable.ic_dislike_outline
                    ),
                    contentDescription = "Dislike level"
                )

                // --- Heart (like) ---
                BottomIconButton(
                    onClick = {
                        syncSuccess.value = false
                        syncError.value = null
                        gameController.toggleThumbUp()
                    },
                    icon = if (currentRating == 1) Icons.Filled.Favorite else Icons.Outlined.FavoriteBorder,
                    contentDescription = "Like level"
                )


                Spacer(modifier = Modifier.weight(1f))

                BottomIconButton(
                    onClick = {
                        syncError.value = null
                        syncSuccess.value = false
                        val handler = Handler(Looper.getMainLooper())
                        Thread {
                            try {
                                gameController.syncWithServer()
                                handler.post {
                                    syncSuccess.value = true
                                }
                            } catch (t: Throwable) {
                                handler.post {
                                    syncError.value = "Sync failed."
                                    syncSuccess.value = false
                                }
                            }
                        }.start()
                    },
                    icon = when {
                        syncSuccess.value -> Icons.Filled._360
                        syncError.value != null -> Icons.Filled.Android
                        else -> Icons.Filled.Sync
                    },
                    contentDescription = "Sync"
                )
            }
        }

        if (gameController.isGameWon) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null
                    ) {
                        gameController.nextLevel()
                    },
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
