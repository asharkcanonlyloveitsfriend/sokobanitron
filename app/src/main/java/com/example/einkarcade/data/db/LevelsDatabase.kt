package com.example.einkarcade.data.db

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import androidx.room.migration.Migration
import androidx.sqlite.db.SupportSQLiteDatabase

@Database(
    entities = [
        LevelSetEntity::class,
        LevelEntity::class,
        PuzzleEntity::class
    ],
    version = 2,
    exportSchema = false
)
abstract class LevelsDatabase : RoomDatabase() {
    abstract fun levelsDao(): LevelsDao

    companion object {
        private val MIGRATION_1_2 = object : Migration(1, 2) {
            override fun migrate(database: SupportSQLiteDatabase) {
                database.execSQL(
                    "ALTER TABLE puzzles ADD COLUMN is_locally_edited INTEGER NOT NULL DEFAULT 0"
                )
            }
        }

        @Volatile
        private var instance: LevelsDatabase? = null
        fun getInstance(context: Context): LevelsDatabase {
            return instance ?: synchronized(this) {
                instance ?: Room.databaseBuilder(
                    context.applicationContext,
                    LevelsDatabase::class.java,
                    "einkarcade.db"
                )
                    .allowMainThreadQueries()
                    .addMigrations(MIGRATION_1_2)
                    .build()
                    .also { instance = it }
            }
        }
    }
}
