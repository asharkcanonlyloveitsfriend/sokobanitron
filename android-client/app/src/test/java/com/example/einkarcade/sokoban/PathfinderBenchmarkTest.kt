package com.example.einkarcade.sokoban

import org.junit.Ignore
import org.junit.Test

@Ignore("Benchmark – run manually")
class PathfinderBenchmarkTest {
    @Test
    fun benchmarkCanFindPath_baseline() {
        val asciiMap =
            """
                   ######
                ####    ###
                #x   ##   #
             #### ###..## ####
             #  #$$     #    #
            ##  $ $ #...   # #
            #  #  $ ##### #  #
            #    ####  #   @##
            ##   #     #   ##
             #####     #####
            """.trimIndent()

        val stats = PathfinderStats()
        val (pathfinder, from, to) = parsePathfinderWithEndpoints(asciiMap, stats)

        repeat(1_000) {
            pathfinder.canFindPath(from, to)
        }

        val start = System.nanoTime()

        repeat(300_000) {
            pathfinder.canFindPath(from, to)
        }

        val elapsedMs = (System.nanoTime() - start) / 1_000_000
        val formattedExpanded = "%,d".format(stats.nodesExpanded)
        val formattedPushed = "%,d".format(stats.nodesPushed)
        println(
            "\n\nelapsedMs     = $elapsedMs\n" +
                "nodesExpanded = $formattedExpanded\n" +
                "nodesPushed   = $formattedPushed\n\n",
        )
    }

    private fun parsePathfinderWithEndpoints(
        asciiMap: String,
        stats: PathfinderStats,
    ): Triple<Pathfinder, Position, Position> {
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

        val pathfinder = createPathfinderFromAscii(asciiMap, stats)
        return Triple(pathfinder, from, to)
    }

    private fun createPathfinderFromAscii(
        asciiMap: String,
        stats: PathfinderStats,
    ): Pathfinder {
        val lines = asciiMap.lines()
        val width = lines.maxOf { it.length }

        val grid =
            lines
                .map { row ->
                    row
                        .padEnd(width, '#')
                        .map { char ->
                            when (char) {
                                '#', '$' -> false
                                ' ', '@', 'x', '.', '*', '+' -> true
                                else -> error("Unsupported character: $char")
                            }
                        }.toTypedArray()
                }.toTypedArray()

        return Pathfinder(grid, stats)
    }
}
