package com.example.einkarcade.catalog

data class LevelSetSummary(
    val id: Int,
    val name: String,
    val levelCount: Int,
    val completedCount: Int,
)

data class LevelSummary(
    val puzzleId: Int,
    val name: String,
    val isCompleted: Boolean,
    val rating: Int,
    val isStarred: Boolean,
    val boardGeometry: LevelBoardGeometry,
)

data class LevelBoardGeometry(
    val rowCount: Int,
    val columnCount: Int,
    val tiles: List<LevelBoardTile>,
    val player: LevelBoardPoint,
    val boxes: List<LevelBoardPoint>,
) {
    fun tileAt(
        row: Int,
        col: Int,
    ): LevelBoardTile = tiles[row * columnCount + col]
}

data class LevelBoardPoint(
    val row: Int,
    val col: Int,
)

enum class LevelBoardTile { FLOOR, GOAL, VOID }

interface LevelCatalog {
    fun getSetSummaries(): List<LevelSetSummary>

    fun getLevelSummaries(setId: Int): List<LevelSummary>

    fun setRating(
        puzzleId: Int,
        rating: Int,
    )

    fun setStarred(
        puzzleId: Int,
        isStarred: Boolean,
    )
}
