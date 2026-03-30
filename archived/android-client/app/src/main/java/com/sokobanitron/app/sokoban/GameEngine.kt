package com.sokobanitron.app.sokoban

class GameEngine(
    level: Level,
) {
    private var nativeHandle: Long = RustGameEngineBridge.create(level.ascii)

    val playerPosition: Position
        get() = RustGameEngineBridge.getPlayerPosition(nativeHandle)

    val boxPositions: Set<Position>
        get() = RustGameEngineBridge.getBoxPositions(nativeHandle)

    val isLevelSolved: Boolean
        get() = RustGameEngineBridge.isLevelSolved(nativeHandle)

    val isCleanSolution: Boolean
        get() = RustGameEngineBridge.isCleanSolution(nativeHandle)

    val isAtStart: Boolean
        get() = RustGameEngineBridge.isAtStart(nativeHandle)

    fun getBoxMoveHistory(): List<List<Position>> = RustGameEngineBridge.getBoxMoveHistory(nativeHandle)

    fun undo(): List<Position>? = RustGameEngineBridge.undo(nativeHandle)

    fun moveBoxTo(
        from: Position,
        to: Position,
    ): List<Position>? {
        if (isLevelSolved) return null
        return RustGameEngineBridge.moveBoxTo(nativeHandle, from, to)
    }

    fun pushBoxIntoVoid(
        from: Position,
        to: Position,
    ): Boolean {
        if (isLevelSolved) return false
        return RustGameEngineBridge.pushBoxIntoVoid(nativeHandle, from, to)
    }

    fun movePlayerTo(position: Position): Boolean {
        if (isLevelSolved) return false
        if (position == playerPosition) return false
        return RustGameEngineBridge.movePlayerTo(nativeHandle, position)
    }

    fun close() {
        if (nativeHandle != 0L) {
            RustGameEngineBridge.destroy(nativeHandle)
            nativeHandle = 0L
        }
    }
}
