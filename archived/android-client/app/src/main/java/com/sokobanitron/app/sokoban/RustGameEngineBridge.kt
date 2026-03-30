package com.sokobanitron.app.sokoban

object RustGameEngineBridge {
    private const val LIB_NAME = "sokobanitron_game_engine_jni"

    @Volatile
    private var loadAttempted = false

    @Volatile
    private var loaded = false

    fun create(levelAscii: String): Long {
        check(ensureLoaded()) {
            "Native library '$LIB_NAME' is not loaded. Build and package libsokobanitron_game_engine_jni.so in jniLibs."
        }
        val handle = nativeCreate(levelAscii)
        check(handle != 0L) { "Failed to create native game engine handle." }
        return handle
    }

    fun destroy(handle: Long) {
        if (!ensureLoaded() || handle == 0L) return
        nativeDestroy(handle)
    }

    fun movePlayerTo(
        handle: Long,
        to: Position,
    ): Boolean = nativeMovePlayerTo(handle, to.row, to.col)

    fun moveBoxTo(
        handle: Long,
        from: Position,
        to: Position,
    ): List<Position>? {
        val flat = nativeMoveBoxTo(handle, from.row, from.col, to.row, to.col)
        if (flat.isEmpty()) return null
        check(flat.size % 2 == 0) { "Native box path payload was invalid." }

        return buildList(flat.size / 2) {
            var index = 0
            while (index < flat.size) {
                add(Position(flat[index], flat[index + 1]))
                index += 2
            }
        }
    }

    fun pushBoxIntoVoid(
        handle: Long,
        from: Position,
        to: Position,
    ): Boolean = nativePushBoxIntoVoid(handle, from.row, from.col, to.row, to.col)

    fun undo(handle: Long): List<Position>? {
        val flat = nativeUndo(handle)
        if (flat.isEmpty()) return null
        check(flat.size % 2 == 0) { "Native undo path payload was invalid." }
        return buildList(flat.size / 2) {
            var index = 0
            while (index < flat.size) {
                add(Position(flat[index], flat[index + 1]))
                index += 2
            }
        }
    }

    fun getPlayerPosition(handle: Long): Position {
        val coords = nativeGetPlayerPosition(handle)
        check(coords.size == 2) { "Native player position payload was invalid." }
        return Position(coords[0], coords[1])
    }

    fun getBoxPositions(handle: Long): Set<Position> {
        val flat = nativeGetBoxPositions(handle)
        check(flat.size % 2 == 0) { "Native box position payload was invalid." }
        return buildSet(flat.size / 2) {
            var index = 0
            while (index < flat.size) {
                add(Position(flat[index], flat[index + 1]))
                index += 2
            }
        }
    }

    fun getBoxMoveHistory(handle: Long): List<List<Position>> {
        val flat = nativeGetBoxMoveHistory(handle)
        if (flat.isEmpty()) return emptyList()

        var index = 0
        val pathCount = flat[index++]
        check(pathCount >= 0) { "Native box move history payload had invalid path count." }

        val history = ArrayList<List<Position>>(pathCount)
        repeat(pathCount) {
            check(index < flat.size) { "Native box move history payload ended before path length." }
            val pathLen = flat[index++]
            check(pathLen >= 0) { "Native box move history payload had invalid path length." }
            check(flat.size - index >= pathLen * 2) {
                "Native box move history payload ended before path coordinates."
            }

            val path = ArrayList<Position>(pathLen)
            repeat(pathLen) {
                val row = flat[index++]
                val col = flat[index++]
                path.add(Position(row, col))
            }
            history.add(path)
        }
        check(index == flat.size) { "Native box move history payload had trailing data." }
        return history
    }

    fun isLevelSolved(handle: Long): Boolean = nativeIsLevelSolved(handle)

    fun isCleanSolution(handle: Long): Boolean = nativeIsCleanSolution(handle)

    fun isAtStart(handle: Long): Boolean = nativeIsAtStart(handle)

    private fun ensureLoaded(): Boolean {
        if (loadAttempted) return loaded
        synchronized(this) {
            if (loadAttempted) return loaded
            loadAttempted = true
            loaded =
                try {
                    System.loadLibrary(LIB_NAME)
                    true
                } catch (_: UnsatisfiedLinkError) {
                    false
                } catch (_: SecurityException) {
                    false
                }
            return loaded
        }
    }

    private external fun nativeCreate(levelAscii: String): Long

    private external fun nativeDestroy(handle: Long)

    private external fun nativeMovePlayerTo(
        handle: Long,
        toRow: Int,
        toCol: Int,
    ): Boolean

    private external fun nativeMoveBoxTo(
        handle: Long,
        fromRow: Int,
        fromCol: Int,
        toRow: Int,
        toCol: Int,
    ): IntArray

    private external fun nativePushBoxIntoVoid(
        handle: Long,
        fromRow: Int,
        fromCol: Int,
        toRow: Int,
        toCol: Int,
    ): Boolean

    private external fun nativeUndo(handle: Long): IntArray

    private external fun nativeGetPlayerPosition(handle: Long): IntArray

    private external fun nativeGetBoxPositions(handle: Long): IntArray

    private external fun nativeGetBoxMoveHistory(handle: Long): IntArray

    private external fun nativeIsLevelSolved(handle: Long): Boolean

    private external fun nativeIsCleanSolution(handle: Long): Boolean

    private external fun nativeIsAtStart(handle: Long): Boolean
}
