package com.example.einkarcade.data.db

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import androidx.room.Transaction

@Dao
interface LevelsDao {
    @Query("SELECT COUNT(*) FROM level_sets")
    fun countLevelSets(): Int

    @Transaction
    @Query("SELECT * FROM level_sets ORDER BY LOWER(title) ASC")
    fun getAllLevelSetsWithLevels(): List<LevelSetWithLevels>

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    fun insertLevelSets(levelSets: List<LevelSetEntity>)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    fun insertLevels(levels: List<LevelEntity>)

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    fun insertPuzzles(puzzles: List<PuzzleEntity>)

    @Query("SELECT * FROM puzzles WHERE is_locally_edited = 1")
    fun getPuzzlesForSync(): List<PuzzleEntity>

    @Query("DELETE FROM levels")
    fun clearLevels()

    @Query("DELETE FROM level_sets")
    fun clearLevelSets()

    @Query("DELETE FROM puzzles")
    fun clearPuzzles()

    @Query("UPDATE puzzles SET rating = :rating, is_locally_edited = 1 WHERE id = :puzzleId")
    fun updatePuzzleRating(
        puzzleId: Int,
        rating: Int,
    )

    @Query("UPDATE puzzles SET is_starred = :isStarred, is_locally_edited = 1 WHERE id = :puzzleId")
    fun updatePuzzleStarred(
        puzzleId: Int,
        isStarred: Boolean,
    )

    @Query(
        "UPDATE puzzles SET last_completed_at = :lastCompletedAt, user_solution = :userSolution, is_locally_edited = 1 WHERE id = :puzzleId",
    )
    fun updatePuzzleCompletion(
        puzzleId: Int,
        lastCompletedAt: String?,
        userSolution: String?,
    )

    @Query("SELECT user_solution FROM puzzles WHERE id = :puzzleId")
    fun getUserSolution(puzzleId: Int): String?
}
