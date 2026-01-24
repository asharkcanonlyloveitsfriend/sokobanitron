package com.example.einkarcade.sokoban

class BoxMover(
    private val staticGrid: Array<Array<Boolean>>
) {
    private data class State(val box: Position, val player: Position)

    fun findBoxPath(from: Position, to: Position, playerStart: Position): List<Position>? {
        if (from == to) return null
        val numRows = staticGrid.size
        val numCols = staticGrid[0].size

        val visited = mutableSetOf<Pair<Position, Position>>()
        val parents = mutableMapOf<State, State?>()
        val queue = ArrayDeque<State>()
        val startState = State(from, playerStart)
        queue.add(startState)
        visited.add(from to playerStart)
        parents[startState] = null

        val directions = listOf(
            Position(-1, 0), Position(1, 0),
            Position(0, -1), Position(0, 1)
        )

        fun isInside(pos: Position): Boolean {
            return pos.row in 0 until numRows && pos.col in 0 until numCols
        }

        while (queue.isNotEmpty()) {
            val (box, player) = queue.removeFirst()
            if (box == to) {
                return buildBoxPath(parents, State(box, player))
            }

            for (dir in directions) {
                val newBox = Position(box.row + dir.row, box.col + dir.col)
                val pushPos = Position(box.row - dir.row, box.col - dir.col)

                if (
                    isInside(newBox) &&
                    isInside(pushPos) &&
                    staticGrid[newBox.row][newBox.col] &&
                    staticGrid[pushPos.row][pushPos.col]
                ) {
                    // Use a fresh grid for each pathfinder instance
                    val pathfinder = pathfinderWithBoxAt(box)
                    if (pathfinder.canFindPath(player, pushPos)) {
                        val newPlayer = box
                        val newState = State(newBox, newPlayer)
                        if ((newBox to newPlayer) !in visited) {
                            visited.add(newBox to newPlayer)
                            parents[newState] = State(box, player)
                            queue.add(newState)
                        }
                    }
                }
            }
        }

        return null
    }

    private fun buildBoxPath(parents: Map<State, State?>, endState: State): List<Position> {
        val reversed = mutableListOf<Position>()
        var current: State? = endState
        while (current != null) {
            reversed.add(current.box)
            current = parents[current]
        }
        reversed.reverse()
        return reversed
    }

    private fun pathfinderWithBoxAt(box: Position): Pathfinder {
        val tempGrid = Array(staticGrid.size) { row ->
            staticGrid[row].copyOf()
        }
        tempGrid[box.row][box.col] = false
        return Pathfinder(tempGrid)
    }
}
