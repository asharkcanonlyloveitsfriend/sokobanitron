package com.example.einkarcade.sokoban

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PathfinderTest {
    @Test
    fun testCanFindPath_StraightLineClear() {
        val asciiMap =
            """
            #####
            #@ x#
            #   #
            #   #
            #####
            """.trimIndent()
        val (pathfinder, from, to) =
            parsePathfinderWithEndpoints(asciiMap).let {
                Triple(
                    it.first,
                    it.second,
                    it.third,
                )
            }

        assertTrue(pathfinder.canFindPath(from, to))
    }

    @Test
    fun testCanFindPath_StraightLineBlockedByWall() {
        val asciiMap =
            """
            #####
            #@#x#
            #   #
            #   #
            #####
            """.trimIndent()
        val (pathfinder, from, to) =
            parsePathfinderWithEndpoints(asciiMap).let {
                Triple(
                    it.first,
                    it.second,
                    it.third,
                )
            }

        assertTrue(pathfinder.canFindPath(from, to))
    }

    @Test
    fun testCanFindPath_MultiTurn() {
        val asciiMap =
            """
            #########
            #@     ##
            ### #   #
            #x# ### #
            #     # #
            #########
            """.trimIndent()
        val (pathfinder, from, to) =
            parsePathfinderWithEndpoints(asciiMap).let {
                Triple(
                    it.first,
                    it.second,
                    it.third,
                )
            }

        assertTrue(pathfinder.canFindPath(from, to))
    }

    @Test
    fun testCanFindPath_TurnCorner() {
        val asciiMap =
            """
            #####
            #@  #
            # # #
            #  x#
            #####
            """.trimIndent()
        val (pathfinder, from, to) =
            parsePathfinderWithEndpoints(asciiMap).let {
                Triple(
                    it.first,
                    it.second,
                    it.third,
                )
            }

        assertTrue(pathfinder.canFindPath(from, to))
    }

    @Test
    fun testCanFindPath_CompletelyBlocked() {
        val asciiMap =
            """
            #######
            #@ # x#
            #######
            """.trimIndent()
        val (pathfinder, from, to) =
            parsePathfinderWithEndpoints(asciiMap).let {
                Triple(
                    it.first,
                    it.second,
                    it.third,
                )
            }

        assertFalse(pathfinder.canFindPath(from, to))
    }

    private fun parsePathfinderWithEndpoints(asciiMap: String): Triple<Pathfinder, Position, Position> {
        var from: Position? = null
        var to: Position? = null

        asciiMap.lines().forEachIndexed { rowIndex, row ->
            row.forEachIndexed { colIndex, char ->
                when (char) {
                    '@' -> from = Position(rowIndex, colIndex)
                    'x' -> to = Position(rowIndex, colIndex)
                }
            }
        }

        require(from != null && to != null) { "Map must contain '@' and 'x'." }

        val pathfinder = createPathfinderFromAscii(asciiMap)
        return Triple(pathfinder, from!!, to!!)
    }

    private fun createPathfinderFromAscii(asciiMap: String): Pathfinder {
        val grid =
            asciiMap
                .lines()
                .map { row ->
                    row
                        .map { char ->
                            when (char) {
                                '#', '$' -> false
                                ' ', '@', 'x' -> true
                                else -> error("Unsupported character: $char")
                            }
                        }.toTypedArray()
                }.toTypedArray()

        return Pathfinder(grid)
    }
}
