package com.sokobanitron.app.sokoban

class BoxPathfinder(
    fullGrid: Array<Array<Boolean>>,
    boxStart: Position,
    playerStart: Position,
    private val stats: BoxPathfinderStats? = null,
) {
    private data class State(
        val box: Position,
        val player: Position,
    )

    private val planningGrid: Array<Array<Boolean>> =
        Array(fullGrid.size) { row ->
            fullGrid[row].copyOf()
        }.also {
            // While planning, the box being moved is treated as walkable
            it[boxStart.row][boxStart.col] = true
        }

    private val startState = State(boxStart, playerStart)
    private val playerPathfinder = Pathfinder(planningGrid)

    companion object {
        private const val DEAD_ENABLE_AFTER_EXPANDED = 16

        private val DIRECTIONS =
            listOf(
                Position(-1, 0),
                Position(1, 0),
                Position(0, -1),
                Position(0, 1),
            )
    }

    fun findBoxPath(to: Position): List<Position>? {
        if (startState.box == to) return null
        val numRows = planningGrid.size
        val numCols = planningGrid[0].size

        fun isInside(pos: Position): Boolean = pos.row in 0 until numRows && pos.col in 0 until numCols

        fun idx(pos: Position): Int = pos.row * numCols + pos.col

        fun computeDeadSquares(goal: Position): BooleanArray {
            val alive = BooleanArray(numRows * numCols)
            val q = ArrayDeque<Position>()

            fun enqueue(p: Position) {
                val i = idx(p)
                if (!alive[i]) {
                    alive[i] = true
                    q.add(p)
                }
            }

            // If the goal itself isn't walkable, nothing is reachable; treat all walkable as dead.
            if (!isInside(goal) || !planningGrid[goal.row][goal.col]) {
                val dead = BooleanArray(numRows * numCols)
                for (r in 0 until numRows) {
                    for (c in 0 until numCols) {
                        if (planningGrid[r][c]) dead[r * numCols + c] = true
                    }
                }
                return dead
            }

            enqueue(goal)

            while (q.isNotEmpty()) {
                val cur = q.removeFirst()

                for (dir in DIRECTIONS) {
                    val prev = Position(cur.row - dir.row, cur.col - dir.col)
                    // In the forward direction, to push `prev -> cur`, the player must stand at `prev - dir`.
                    val pushPos = Position(prev.row - dir.row, prev.col - dir.col)

                    if (!isInside(prev) || !isInside(pushPos)) continue
                    if (!planningGrid[prev.row][prev.col]) continue
                    if (!planningGrid[pushPos.row][pushPos.col]) continue

                    enqueue(prev)
                }
            }

            val dead = BooleanArray(numRows * numCols)
            for (r in 0 until numRows) {
                for (c in 0 until numCols) {
                    val p = Position(r, c)
                    val i = idx(p)
                    if (planningGrid[r][c] && !alive[i] && p != goal) {
                        dead[i] = true
                    }
                }
            }
            return dead
        }

        // Only pay for dead-square computation once the search proves non-trivial.
        // micro94 expands 3 states; typical harder cases expand 50+.

        var dead: BooleanArray? = null
        var expandedCount = 0

        val visited = mutableSetOf<State>()
        val parents = mutableMapOf<State, State?>()
        val queue = ArrayDeque<State>()
        queue.add(startState)
        stats?.statesPushed = stats?.statesPushed?.plus(1) ?: 0
        visited.add(startState)
        parents[startState] = null

        fun isWalkable(pos: Position): Boolean = planningGrid[pos.row][pos.col]

        fun canAttemptPush(
            newBox: Position,
            pushPos: Position,
        ): Boolean =
            isInside(newBox) &&
                isInside(pushPos) &&
                isWalkable(newBox) &&
                isWalkable(pushPos)

        fun enqueueIfNew(
            newState: State,
            parent: State,
        ) {
            if (newState !in visited) {
                visited.add(newState)
                parents[newState] = parent
                queue.add(newState)
                stats?.statesPushed = stats?.statesPushed?.plus(1) ?: 0
            }
        }

        while (queue.isNotEmpty()) {
            val (box, player) = queue.removeFirst()
            stats?.statesExpanded = stats?.statesExpanded?.plus(1) ?: 0
            expandedCount += 1
            if (dead == null && expandedCount >= DEAD_ENABLE_AFTER_EXPANDED) {
                dead = computeDeadSquares(to)
            }
            if (box == to) {
                return buildBoxPath(parents, State(box, player))
            }

            for (dir in DIRECTIONS) {
                stats?.pushAttempts = stats?.pushAttempts?.plus(1) ?: 0
                val newBox = Position(box.row + dir.row, box.col + dir.col)
                val pushPos = Position(box.row - dir.row, box.col - dir.col)

                // Static dead-square prune (enabled lazily): if the box can't ever reach the goal from here, skip.
                val deadGrid = dead
                if (deadGrid != null && isInside(newBox) && deadGrid[idx(newBox)]) continue

                if (canAttemptPush(newBox, pushPos)) {
                    stats?.playerPathfinderCalls = stats?.playerPathfinderCalls?.plus(1) ?: 0
                    if (playerPathfinder.canFindPath(player, pushPos, blocked = box)) {
                        stats?.playerPathfinderSuccesses =
                            stats?.playerPathfinderSuccesses?.plus(1) ?: 0
                        val newPlayer = box
                        val newState = State(newBox, newPlayer)
                        enqueueIfNew(newState, State(box, player))
                    }
                }
            }
        }

        return null
    }

    private fun buildBoxPath(
        parents: Map<State, State?>,
        endState: State,
    ): List<Position> {
        val reversed = mutableListOf<Position>()
        var current: State? = endState
        while (current != null) {
            reversed.add(current.box)
            current = parents[current]
        }
        reversed.reverse()
        return reversed
    }
}
