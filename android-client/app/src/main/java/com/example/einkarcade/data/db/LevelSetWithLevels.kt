package com.example.einkarcade.data.db

import androidx.room.Embedded
import androidx.room.Relation

data class LevelWithPuzzle(
    @Embedded val level: LevelEntity,
    @Relation(
        parentColumn = "puzzle_id",
        entityColumn = "id",
    )
    val puzzle: PuzzleEntity,
)

data class LevelSetWithLevels(
    @Embedded val levelSet: LevelSetEntity,
    @Relation(
        parentColumn = "id",
        entityColumn = "level_set_id",
        entity = LevelEntity::class,
    )
    val levels: List<LevelWithPuzzle>,
)
