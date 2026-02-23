package com.sokobanitron.app

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.click
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performTouchInput
import com.sokobanitron.app.content.LevelSet
import com.sokobanitron.app.sokoban.Level
import com.sokobanitron.app.ui.rendering.geom.BoardViewport
import com.sokobanitron.app.ui.rendering.geom.computeBoardViewport
import org.junit.After
import org.junit.Rule
import org.junit.Test

class MainActivityTest {
    companion object {
        init {
            MainActivity.gameControllerFactory = { ctx ->
                GameController(
                    ctx,
                    listOf(
                        LevelSet(
                            id = 1,
                            name = "Training",
                            levels =
                                listOf(
                                    Level.fromAscii(
                                        "Level 1",
                                        """
                                        ####
                                        #@$.#
                                        ####
                                        """.trimIndent(),
                                        puzzleId = 101,
                                    ),
                                    Level.fromAscii(
                                        "Level 2",
                                        """
                                        #####
                                        #@ $.#
                                        #####
                                        """.trimIndent(),
                                        puzzleId = 102,
                                    ),
                                ),
                        ),
                    ),
                )
            }
        }
    }

    @After
    fun tearDown() {
        MainActivity.gameControllerFactory = null
    }

    @get:Rule
    val composeTestRule = createAndroidComposeRule<MainActivity>()

    @Test
    fun playerSolvesLevelAndAdvancesToNextLevel() {
        composeTestRule
            .onNodeWithText("Level 1", substring = true)
            .assertIsDisplayed()

        composeTestRule.onNodeWithTag("gameCanvas").performTouchInput {
            val viewport =
                computeBoardViewport(
                    surfaceWidth = visibleSize.width.toFloat(),
                    surfaceHeight = visibleSize.height.toFloat(),
                    innerRows = 3,
                    innerCols = 5,
                )
            click(gridOffsetInMiddleRow(viewport = viewport, col = 2))
            click(gridOffsetInMiddleRow(viewport = viewport, col = 3))
        }

        composeTestRule
            .onNodeWithTag("levelSolvedView")
            .assertIsDisplayed()

        composeTestRule.onNodeWithTag("levelSolvedView").performTouchInput {
            click(
                Offset(
                    x = visibleSize.width * 0.95f,
                    y = visibleSize.height * 0.1f,
                ),
            )
        }

        composeTestRule.onNodeWithTag("gameCanvas").performTouchInput {
            click(
                Offset(
                    x = visibleSize.width * 0.95f,
                    y = visibleSize.height * 0.1f,
                ),
            )
        }
        composeTestRule.waitForIdle()
        composeTestRule.onNodeWithText("Level 2", substring = true).assertIsDisplayed()
    }
}

private fun gridOffsetInMiddleRow(
    viewport: BoardViewport,
    col: Int,
): Offset =
    Offset(
        viewport.offsetX + viewport.cellSize * (col + 1.5f),
        viewport.offsetY + viewport.cellSize * (1 + 1.5f),
    )
