package com.example.einkarcade.sokoban

// import org.junit.Ignore
import org.junit.Test

// @Ignore("Benchmark - run manually")
class BoxPathfinderBenchmarkTest {
    @Test
    fun benchmarkFindBoxPath_baseline() {
        val cases =
            listOf(
                Triple(
                    "l5",
                    """
                    #######
                    #     #
                    #    x#
                    #     #
                    # b   #
                    #@    #
                    #######
                    """.trimIndent(),
                    100_000,
                ),
                Triple(
                    "micro94",
                    """
                           ######                   
                        ####    ###
                        #x   ##   #
                     #### ###..## ####
                     #  #b$     #    #
                    ##  $ $ #...   # #
                    #  #  $ ##### #  #
                    #    ####  #    ##
                    ## @ #     #   ##
                     #####     #####
                    """.trimIndent(),
                    100_000,
                ),
                Triple(
                    "misc19",
                    """
                    #####          ####
                    #   ############  #
                    #             x   #
                    ## #############  #
                     # #  ######   # ##
                     # #  $  @   $ # #
                     # #  # ### #### #
                     # ## # #.# #  # #
                     # ## # #.# #  # #
                     # ## # #.# #  # #
                     # ## # #.# #  # #
                     # ## # #.# #  # #
                    ## ## b $ # #### ##
                    #  #    # #       #
                    #         $ ###   #
                    #  ########## #####
                    ####
                    """.trimIndent(),
                    3_000,
                ),
                Triple(
                    "misc22",
                    """
                    ##################################
                    #           #    ##             ##
                    #  $  $ $ b@  #                 ##
                    ## ########## ## ############# ###
                    #  ########## ## #############.  #
                    # $########## ##        ######   #
                    #  ########## ## #####x ######   #
                    # ########### ## #####. #######  #
                    # ########### ## #####. #######  #
                    # ########### ## #####. #######  #
                    # ########### #  ##### ########  #
                    # ########### #        ########  #
                    # ########### #######  ########  #
                    # ########### #################  #
                    # ########### #################  #
                    # ########### #################  #
                    # ########### #################  #
                    # ##########                     #
                    #             #################  #
                    ##################################
                    """.trimIndent(),
                    1_000,
                ),
                Triple(
                    "sas27",
                    """
                          ########
                     ######      ##########
                    ## $     ###          ##
                    # $$  ## #  #########  #
                    #  $  #              # #
                    # $ $ # #  ######### # #
                    # $   #  # #. . . .  # #
                    # $ b # #    .x. . .## #
                    #  $  # # # . . . . #  #
                    ##$ $## #  #### # #   ##
                     #   #@ #       #  ####
                     ######  #####  #  #
                          ##     #  #  #
                           #####  ##  ##
                               ##    ##
                                ######
                    """.trimIndent(),
                    2_000,
                ),
            )

        println()
        println("CASE        path   ns/run   states   pushes   calls   success")
        println("------------------------------------------------------------")

        for ((name, asciiMap, iterations) in cases) {
            val (pathfinder, to, stats) = parseBoxPathfinderWithTarget(asciiMap)

            val baselinePath =
                requireNotNull(pathfinder.findBoxPath(to)) {
                    "Case $name must produce a non-null box path."
                }

            // warm
            repeat(1_000) {
                pathfinder.findBoxPath(to)
            }

            stats.statesPushed = 0
            stats.statesExpanded = 0
            stats.pushAttempts = 0
            stats.playerPathfinderCalls = 0
            stats.playerPathfinderSuccesses = 0

            val start = System.nanoTime()

            repeat(iterations) {
                pathfinder.findBoxPath(to)
            }

            val elapsedNs = System.nanoTime() - start
            val nanosPerRun = elapsedNs / iterations

            println(
                "%-10s %5d %8d %8d %8d %8d %8d".format(
                    name,
                    baselinePath.size,
                    nanosPerRun,
                    stats.statesExpanded / iterations,
                    stats.pushAttempts / iterations,
                    stats.playerPathfinderCalls / iterations,
                    stats.playerPathfinderSuccesses / iterations,
                ),
            )
        }

        println()
    }

    private fun parseBoxPathfinderWithTarget(asciiMap: String): Triple<BoxPathfinder, Position, BoxPathfinderStats> {
        val lines = asciiMap.lines()
        val width = lines.maxOf { it.length }
        var player: Position? = null
        var to: Position? = null
        var box: Position? = null

        val grid =
            lines
                .mapIndexed { rowIndex, row ->
                    row
                        .padEnd(width, '#')
                        .mapIndexed { colIndex, char ->
                            when (char) {
                                '#', '$' -> {
                                    false
                                }

                                'b' -> {
                                    box = Position(rowIndex, colIndex)
                                    true
                                }

                                '@' -> {
                                    player = Position(rowIndex, colIndex)
                                    true
                                }

                                'x' -> {
                                    to = Position(rowIndex, colIndex)
                                    true
                                }

                                ' ', '.', '*', '+' -> {
                                    true
                                }

                                else -> {
                                    error("Unsupported character: $char")
                                }
                            }
                        }.toTypedArray()
                }.toTypedArray()

        require(player != null && to != null && box != null) {
            "Map must contain '@', 'x', and 'b'."
        }

        val stats = BoxPathfinderStats()
        return Triple(
            BoxPathfinder(
                fullGrid = grid,
                boxStart = box,
                playerStart = player,
                stats = stats,
            ),
            to,
            stats,
        )
    }
}
