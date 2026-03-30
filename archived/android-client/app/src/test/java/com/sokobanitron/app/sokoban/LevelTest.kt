package com.sokobanitron.app.sokoban

import org.junit.Assert.assertEquals
import org.junit.Test

class LevelTest {
    @Test
    fun testFromAscii_ParsesGridAndExtractsEntities() {
        val ascii =
            """
            #######
            #@ $. #
            #   . #
            #######
            """.trimIndent()

        val level = Level.fromAscii("Rectangular", ascii)

        assertEquals(4, level.grid.size)
        assertEquals(7, level.grid.first().size)
        assertEquals(Position(1, 1), level.playerStart)
        assertEquals(setOf(Position(1, 3)), level.boxPositions)
        assertEquals(Tile.GOAL, level.grid[2][4])
    }
}
