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
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowForward
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Favorite
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Warning
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
import com.example.einkarcade.ui.rendering.drawBoxPathLine
import com.example.einkarcade.ui.rendering.drawBox
import com.example.einkarcade.ui.rendering.drawVanishingBox
import com.example.einkarcade.ui.rendering.drawFloor
import com.example.einkarcade.ui.rendering.drawGoal
import com.example.einkarcade.ui.rendering.drawPlayer
import kotlinx.coroutines.delay
import kotlin.math.min


@Composable
fun GameScreen(
    modifier: Modifier = Modifier,
    gameController: GameController,
    selectedBoxPosition: MutableState<Position?>
) {
    gameController.revision.value
    val playerPosition = gameController.playerPosition
    val boxPathAnimation = rememberBoxPathAnimationState()
    val displayedPlayerPosition = boxPathAnimation.displayedPlayerPosition(playerPosition)
    val syncError = remember { mutableStateOf<String?>(null) }
    val syncSuccess = remember { mutableStateOf(false) }
    val lastBackTapTime = remember { mutableStateOf<Long?>(null) }
    val doubleTapWindowMs = 350L
    val boxPathShrink = boxPathAnimation.shrink
    val boxPathActive = boxPathAnimation.isActive
    val boxPathPositions = boxPathAnimation.path
    val boxPainter = painterResource(id = R.drawable.box)
    val selectedBoxPainter = painterResource(id = R.drawable.box_selected)
    val playerPainter = painterResource(id = R.drawable.player_slime)
    val openEyesPainter = painterResource(id = R.drawable.player_eyes_open)
    val blinkEyesPainter = painterResource(id = R.drawable.player_eyes_blink)
    val focusRequester = remember { FocusRequester() }
    val vanishAnimation = rememberVanishAnimationState()

    val isBlinking = remember { mutableStateOf(false) }
    val blinkPulse = remember { mutableStateOf(0) }
    val isFacingLeft = remember { mutableStateOf(false) }
    val currentSetName = gameController.currentSetName
    val currentLevelName = gameController.levelName

    fun resetSelectionAndFacing() {
        selectedBoxPosition.value = null
        isFacingLeft.value = false
    }

    BackHandler(enabled = true) {
        // handled manually via key events below
    }

    LaunchedEffect(Unit) {
        focusRequester.requestFocus()
    }

    LaunchedEffect(blinkPulse.value) {
        if (blinkPulse.value == 0) return@LaunchedEffect
        delay(400L)
        isBlinking.value = true
        delay(300L)
        isBlinking.value = false
    }

    LaunchedEffect(currentSetName, currentLevelName) {
        resetSelectionAndFacing()
    }

    Box(
        modifier = modifier
            .fillMaxSize()
            .focusRequester(focusRequester)
            .focusable()
            .onKeyEvent { event ->
                if (event.key == Key.Back) {
                    when (event.type) {
                        KeyEventType.KeyDown -> true
                        KeyEventType.KeyUp -> {
                            val now = SystemClock.elapsedRealtime()
                            val lastTap = lastBackTapTime.value
                            if (lastTap != null && now - lastTap <= doubleTapWindowMs) {
                                lastBackTapTime.value = null
                                resetSelectionAndFacing()
                                gameController.restart()
                            } else {
                                lastBackTapTime.value = now
                                isFacingLeft.value = false
                                gameController.undo()
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
        fun handleTap(tappedPosition: Position) {
            fun attemptBoxMove(selectedBox: Position) {
                val boxPath = gameController.moveBoxTo(selectedBox, tappedPosition)
                if (boxPath == null) {
                    blinkPulse.value += 1
                    return
                }
                val previous = boxPath[boxPath.size - 2]
                val current = boxPath.last()
                val pushLeft = previous.row == current.row && current.col < previous.col
                if (!pushLeft) {
                    isFacingLeft.value = false
                }
                val lastPosition = boxPath.last()
                if (gameController.tiles[lastPosition.row][lastPosition.col] == Tile.WALL) {
                    vanishAnimation.start(lastPosition)
                    blinkPulse.value += 1
                }
                boxPathAnimation.start(boxPath, gameController.playerPosition) {
                    isFacingLeft.value = pushLeft
                }
            }

            val tile = gameController.tiles[tappedPosition.row][tappedPosition.col]
            val selectedBox = selectedBoxPosition.value

            if (tile == Tile.WALL) {
                if (selectedBox != null) {
                    selectedBoxPosition.value = null
                    attemptBoxMove(selectedBox)
                }
                return
            }

            if (gameController.boxPositions.contains(tappedPosition)) {
                if (selectedBox == tappedPosition) {
                    selectedBoxPosition.value = null
                } else {
                    selectedBoxPosition.value = tappedPosition
                }
            } else if (selectedBox != null) {
                selectedBoxPosition.value = null
                attemptBoxMove(selectedBox)
            } else {
                gameController.movePlayerTo(tappedPosition)
                isFacingLeft.value = false
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
                        text = currentSetName,
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
                                val isSelected = name == currentSetName
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
                val selectedLevelIndex = levels.indexOfFirst { it.name == currentLevelName }
                val levelScrollState = rememberScrollState()
                val density = LocalDensity.current
                val itemHeight: Dp = 40.dp

                Box(
                    modifier = Modifier
                        .clickable { levelExpanded.value = true }
                ) {
                    Text(
                        text = currentLevelName,
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
                                drawVanishingBox(
                                    vanish = vanishAnimation.state.value,
                                    gridPosition = Position(rowIndex, colIndex),
                                    paddedPosition = Position(paddedRow, paddedCol),
                                    boxPainter = boxPainter,
                                    selectedBoxPainter = selectedBoxPainter,
                                    cellSize = cellSize,
                                    offsetX = offsetX,
                                    offsetY = offsetY
                                )
                            }
                        }
                    }
                }

                drawBoxPathLine(
                    isActive = boxPathActive.value,
                    shrink = boxPathShrink.value,
                    path = boxPathPositions.value,
                    cellSize = cellSize,
                    offsetX = offsetX,
                    offsetY = offsetY
                )

                for (position in gameController.boxPositions) {
                    drawBox(
                        Position(position.row + 1, position.col + 1),
                        boxPainter,
                        selectedBoxPainter,
                        position == selectedBoxPosition.value,
                        cellSize,
                        offsetX,
                        offsetY
                    )
                }

                val drawnPlayerPosition = displayedPlayerPosition
                val flipPlayer = isFacingLeft.value
                drawPlayer(
                    Position(drawnPlayerPosition.row + 1, drawnPlayerPosition.col + 1),
                    playerPainter,
                    flipPlayer,
                    cellSize,
                    offsetX,
                    offsetY
                )
                drawPlayer(
                    Position(drawnPlayerPosition.row + 1, drawnPlayerPosition.col + 1),
                    if (isBlinking.value) blinkEyesPainter else openEyesPainter,
                    flipPlayer,
                    cellSize,
                    offsetX,
                    offsetY
                )
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
                    onClick = {
                        gameController.previousLevel()
                    },
                    icon = Icons.Filled.ArrowBack,
                    contentDescription = "Previous level"
                )

                BottomIconButton(
                    onClick = {
                        gameController.nextLevel()
                    },
                    icon = Icons.Filled.ArrowForward,
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
                        syncSuccess.value -> Icons.Filled.Check
                        syncError.value != null -> Icons.Filled.Warning
                        else -> Icons.Filled.Refresh
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
