package com.sokobanitron.app.sokoban

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class BoxPathfinderTest {
    data class TripleResult<A, B, C>(
        val first: A,
        val second: B,
        val third: C,
    )

    @Test
    fun testFindBoxPath_StraightLineWithPlayerAccess() {
        val asciiMap =
            """
            #######
            #@    #
            # $  x#
            #######
            """.trimIndent()
        val (mover, to, _) = parseBoxMoverWithEndpoints(asciiMap)

        val expectedPath =
            listOf(
                Position(2, 2),
                Position(2, 3),
                Position(2, 4),
                Position(2, 5),
            )
        val path = mover.findBoxPath(to)
        assertNotNull(path)
        assertEquals(expectedPath, path)
    }

    @Test
    fun testFindBoxPath_NotStraightLine() {
        val asciiMap =
            """
            #####
            #@  #
            # $ #
            #  x#
            #####
            """.trimIndent()
        val (mover, to, _) = parseBoxMoverWithEndpoints(asciiMap)

        val expectedPath =
            listOf(
                Position(2, 2),
                Position(3, 2),
                Position(3, 3),
            )
        val path = mover.findBoxPath(to)
        assertNotNull(path)
        assertEquals(expectedPath, path)
    }

    @Test
    fun testFindBoxPath_ComplexPath() {
        val asciiMap =
            """
            ###################
            # ###   ##        #
            # #  $#  #        #
            ### # ## #   ######
            #   # ## ## ##    #
            # #              ##
            #    ####    @#  x#
            ###################
            """.trimIndent()
        val (mover, to, boxPosition) = parseBoxMoverWithEndpoints(asciiMap)

        val path = mover.findBoxPath(to)
        assertNotNull(path)
        assertTrue(path!!.isNotEmpty())
        assertEquals(boxPosition, path.first())
        assertEquals(to, path.last())
    }

    @Test
    fun testFindBoxPath_Blocked() {
        val asciiMap =
            """
            #####
            #   #
            ###$#
            #  @#
            # #x#
            #####
            """.trimIndent()
        val (mover, to, _) = parseBoxMoverWithEndpoints(asciiMap)

        assertNull(mover.findBoxPath(to))
    }

    private fun parseBoxMoverWithEndpoints(asciiMap: String): TripleResult<BoxPathfinder, Position, Position> {
        var player: Position? = null
        var to: Position? = null
        var box: Position? = null

        val grid =
            asciiMap
                .lines()
                .mapIndexed { rowIndex, row ->
                    row
                        .mapIndexed { colIndex, char ->
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

                                '#' -> {
                                    false
                                }

                                ' ' -> {
                                    true
                                }

                                else -> {
                                    error("Unsupported character: $char")
                                }
                            }
                        }.toTypedArray()
                }.toTypedArray()

        require(player != null && to != null && box != null) {
            "Map must contain '@', 'x', and '\$'."
        }

        val mover =
            BoxPathfinder(
                fullGrid = grid,
                boxStart = box,
                playerStart = player,
            )
        return TripleResult(mover, to!!, box!!)
    }
}
