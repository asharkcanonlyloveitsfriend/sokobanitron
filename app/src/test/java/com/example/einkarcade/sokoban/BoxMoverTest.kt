package com.example.einkarcade.sokoban

import org.junit.Assert.*
import org.junit.Test

class BoxMoverTest {

    data class Quadruple<A, B, C, D>(val first: A, val second: B, val third: C, val fourth: D)

    @Test
    fun testFindBoxPath_StraightLineWithPlayerAccess() {
        val asciiMap = """
            #######
            #@    #
            # $  x#
            #######
        """.trimIndent()
        val (mover, playerPosition, to, boxPosition) = parseBoxMoverWithEndpoints(asciiMap)

        val expectedPath = listOf(
            Position(2, 2),
            Position(2, 3),
            Position(2, 4),
            Position(2, 5)
        )
        val path = mover.findBoxPath(boxPosition, to, playerPosition)
        assertNotNull(path)
        assertEquals(expectedPath, path)
    }
    @Test
    fun testFindBoxPath_NotStraightLine() {
        val asciiMap = """
            #####
            #@  #
            # $ #
            #  x#
            #####
        """.trimIndent()
        val (mover, playerPosition, to, boxPosition) = parseBoxMoverWithEndpoints(asciiMap)

        val expectedPath = listOf(
            Position(2, 2),
            Position(3, 2),
            Position(3, 3)
        )
        val path = mover.findBoxPath(boxPosition, to, playerPosition)
        assertNotNull(path)
        assertEquals(expectedPath, path)
    }
    @Test
    fun testFindBoxPath_ComplexPath() {
        val asciiMap = """
            ###################
            # ###   ##        #
            # #  $#  #        #
            ### # ## #   ######
            #   # ## ## ##    #
            # #              ##
            #    ####    @#  x#
            ###################
        """.trimIndent()
        val (mover, playerPosition, to, boxPosition) = parseBoxMoverWithEndpoints(asciiMap)

        val path = mover.findBoxPath(boxPosition, to, playerPosition)
        assertNotNull(path)
        assertTrue(path!!.isNotEmpty())
        assertEquals(boxPosition, path.first())
        assertEquals(to, path.last())
    }
    @Test
    fun testFindBoxPath_Blocked() {
        val asciiMap = """
            #####
            #   #
            ###$#
            #  @#
            # #x#
            #####
        """.trimIndent()
        val (mover, playerPosition, to, boxPosition) = parseBoxMoverWithEndpoints(asciiMap)

        assertNull(mover.findBoxPath(boxPosition, to, playerPosition))
    }

    private fun parseBoxMoverWithEndpoints(asciiMap: String): Quadruple<BoxMover, Position, Position, Position> {
        var player: Position? = null
        var to: Position? = null
        var box: Position? = null

        val grid = asciiMap.lines().mapIndexed { rowIndex, row ->
            row.mapIndexed { colIndex, char ->
                when (char) {
                    '@' -> {
                        player = Position(rowIndex, colIndex)
                        true
                    }
                    'x' -> {
                        to = Position(rowIndex, colIndex)
                        true
                    }
                    '$' -> {
                        box = Position(rowIndex, colIndex)
                        true
                    }
                    '#' -> false
                    ' ' -> true
                    else -> error("Unsupported character: $char")
                }
            }.toTypedArray()
        }.toTypedArray()

        require(player != null && to != null && box != null) {
            "Map must contain '@', 'x', and '\$'."
        }

        val mover = BoxMover(grid)
        return Quadruple(mover, player!!, to!!, box!!)
    }

}