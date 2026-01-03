package com.example.einkarcade.ui.screens

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.focusable
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
import com.example.einkarcade.ui.rendering.ComposeGameAssets
import com.example.einkarcade.ui.rendering.ComposeGameBoard
import com.example.einkarcade.ui.rendering.SurfaceGameBoard
import com.example.einkarcade.ui.rendering.buildGameScene
import kotlinx.coroutines.delay


@Composable
fun GameScreen(
    modifier: Modifier = Modifier,
    gameController: GameController,
    selectedBoxPosition: MutableState<Position?>
) {
    val useSurfaceView = false
    gameController.revision.value
    val animRevision = remember { mutableStateOf(0) }
    animRevision.value
    val playerPosition = gameController.playerPosition
    val boxPathAnimation = remember { BoxPathAnimator() }
    val displayedPlayerPosition = boxPathAnimation.displayedPlayerPosition(playerPosition)
    val syncError = remember { mutableStateOf<String?>(null) }
    val syncSuccess = remember { mutableStateOf(false) }
    val boxPainter = painterResource(id = R.drawable.box)
    val selectedBoxPainter = painterResource(id = R.drawable.box_selected)
    val playerPainter = painterResource(id = R.drawable.player_slime)
    val openEyesPainter = painterResource(id = R.drawable.player_eyes_open)
    val blinkEyesPainter = painterResource(id = R.drawable.player_eyes_blink)
    val assets = ComposeGameAssets(
        boxPainter = boxPainter,
        selectedBoxPainter = selectedBoxPainter,
        playerPainter = playerPainter,
        openEyesPainter = openEyesPainter,
        blinkEyesPainter = blinkEyesPainter
    )
    val focusRequester = remember { FocusRequester() }
    val vanishAnimation = remember { VanishAnimator() }
    val ui = remember { GameUiState(selectedBox = selectedBoxPosition.value) }
    val anim = remember { GameAnimState(boxPathAnimation, vanishAnimation) }
    val currentSetName = gameController.currentSetName
    val currentLevelName = gameController.levelName

    fun resetSelectionAndFacing() {
        selectedBoxPosition.value = null
        ui.selectedBox = null
        ui.isFacingLeft = false
    }

    BackHandler(enabled = true) {
        // handled manually via key events below
    }

    LaunchedEffect(Unit) {
        focusRequester.requestFocus()
    }

    LaunchedEffect(Unit) {
        var wasActive = false
        while (true) {
            val now = SystemClock.elapsedRealtime()

            boxPathAnimation.update(now)
            vanishAnimation.update(now)

            val blinkActive = now < ui.blinkEndMs
            val active = boxPathAnimation.isActive || vanishAnimation.state != null || blinkActive

            if (active) {
                animRevision.value += 1
                delay(16L)
            } else {
                if (wasActive) {
                    // Ensure a final recomposition after an animation/blink completes.
                    animRevision.value += 1
                }
                delay(100L)
            }

            wasActive = active
        }
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
                            val nowMs = SystemClock.elapsedRealtime()
                            ui.selectedBox = selectedBoxPosition.value
                            GameInputHandler.handleBackKeyUp(
                                nowMs = nowMs,
                                gameController = gameController,
                                ui = ui,
                                resetSelection = {
                                    selectedBoxPosition.value = null
                                    ui.selectedBox = null
                                    ui.isFacingLeft = false
                                }
                            )
                            selectedBoxPosition.value = ui.selectedBox
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

            val nowForBlink = SystemClock.elapsedRealtime()
            val blinking = ui.isBlinking(nowForBlink)
            val scene = buildGameScene(
                gameController = gameController,
                ui = ui,
                displayedPlayerPosition = displayedPlayerPosition,
                isBlinking = blinking,
                boxPathAnimation = boxPathAnimation,
                vanishAnimation = vanishAnimation
            ).copy(selectedBox = selectedBoxPosition.value)
            if (useSurfaceView) {
                SurfaceGameBoard(
                    scene = scene,
                    isGameWon = gameController.isGameWon,
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .testTag("gameCanvas"),
                    onTapCell = { pos ->
                        val nowMs = SystemClock.elapsedRealtime()
                        ui.selectedBox = selectedBoxPosition.value
                        GameInputHandler.handleTap(
                            tappedPosition = pos,
                            nowMs = nowMs,
                            gameController = gameController,
                            ui = ui,
                            anim = anim
                        )
                        selectedBoxPosition.value = ui.selectedBox
                    }
                )
            } else {
                ComposeGameBoard(
                    scene = scene,
                    assets = assets,
                    isGameWon = gameController.isGameWon,
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .testTag("gameCanvas"),
                    onTapCell = { pos ->
                        val nowMs = SystemClock.elapsedRealtime()
                        ui.selectedBox = selectedBoxPosition.value
                        GameInputHandler.handleTap(
                            tappedPosition = pos,
                            nowMs = nowMs,
                            gameController = gameController,
                            ui = ui,
                            anim = anim
                        )
                        selectedBoxPosition.value = ui.selectedBox
                    }
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
