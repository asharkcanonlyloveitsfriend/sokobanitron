package com.sokobanitron.app.catalog

import android.content.Context
import com.sokobanitron.app.content.LevelSet
import com.sokobanitron.app.data.LevelsRepository
import com.sokobanitron.app.sokoban.Level
import com.sokobanitron.app.sokoban.Tile

class RepositoryLevelCatalog(
    context: Context,
    private val injectedSets: List<LevelSet>? = null,
) : LevelCatalog {
    private val repository = LevelsRepository(context)

    private fun loadSetsFresh(): List<LevelSet> = injectedSets ?: (repository.loadSets() ?: emptyList())

    override fun getSetSummaries(): List<LevelSetSummary> {
        val sets = loadSetsFresh()
        return sets.map { set ->
            LevelSetSummary(
                id = set.id,
                name = set.name,
                levelCount = set.levels.size,
                completedCount = set.levels.count { it.isCompleted },
            )
        }
    }

    override fun getLevelSummaries(setId: Int): List<LevelSummary> {
        val sets = loadSetsFresh()
        val set = sets.firstOrNull { it.id == setId } ?: return emptyList()
        return set.levels.map { level ->
            LevelSummary(
                puzzleId = level.puzzleId,
                name = level.name,
                isCompleted = level.isCompleted,
                rating = level.rating,
                isStarred = level.isStarred,
                boardGeometry = level.toBoardGeometry(),
            )
        }
    }

    override fun setRating(
        puzzleId: Int,
        rating: Int,
    ) {
        val sets = loadSetsFresh()
        val level = findLevelByPuzzleId(sets, puzzleId) ?: return
        level.setRating(rating)
        repository.updateRating(level)
    }

    override fun setStarred(
        puzzleId: Int,
        isStarred: Boolean,
    ) {
        val sets = loadSetsFresh()
        val level = findLevelByPuzzleId(sets, puzzleId) ?: return
        level.setStarred(isStarred)
        repository.updateStarred(level)
    }

    private fun findLevelByPuzzleId(
        sets: List<LevelSet>,
        puzzleId: Int,
    ): Level? {
        sets.forEach { set ->
            set.levels.firstOrNull { it.puzzleId == puzzleId }?.let { return it }
        }
        return null
    }

    private fun Level.toBoardGeometry(): LevelBoardGeometry {
        val rowCount = grid.size
        val columnCount = grid.firstOrNull()?.size ?: 0
        val tiles =
            grid.flatMap { row ->
                row.map { tile ->
                    when (tile) {
                        Tile.FLOOR -> LevelBoardTile.FLOOR
                        Tile.GOAL -> LevelBoardTile.GOAL
                        Tile.VOID -> LevelBoardTile.VOID
                    }
                }
            }

        return LevelBoardGeometry(
            rowCount = rowCount,
            columnCount = columnCount,
            tiles = tiles,
            player = LevelBoardPoint(playerStart.row, playerStart.col),
            boxes = boxPositions.map { LevelBoardPoint(it.row, it.col) }.sortedWith(compareBy({ it.row }, { it.col })),
        )
    }
}
