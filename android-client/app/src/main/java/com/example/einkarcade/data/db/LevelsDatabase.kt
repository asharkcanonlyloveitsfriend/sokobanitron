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
        PuzzleEntity::class,
    ],
    version = 4,
    exportSchema = false,
)
abstract class LevelsDatabase : RoomDatabase() {
    abstract fun levelsDao(): LevelsDao

    companion object {
        private val MIGRATION_1_2 =
            object : Migration(1, 2) {
                override fun migrate(database: SupportSQLiteDatabase) {
                    database.execSQL(
                        "ALTER TABLE puzzles ADD COLUMN is_locally_edited INTEGER NOT NULL DEFAULT 0",
                    )
                }
            }

        private val MIGRATION_2_3 =
            object : Migration(2, 3) {
                override fun migrate(database: SupportSQLiteDatabase) {
                    database.execSQL(
                        "ALTER TABLE puzzles ADD COLUMN user_solution TEXT",
                    )
                }
            }

        private val MIGRATION_3_4 =
            object : Migration(3, 4) {
                override fun migrate(database: SupportSQLiteDatabase) {
                    database.execSQL(
                        "ALTER TABLE puzzles ADD COLUMN is_starred INTEGER NOT NULL DEFAULT 0",
                    )
                }
            }

        @Volatile
        private var instance: LevelsDatabase? = null

        fun getInstance(context: Context): LevelsDatabase =
            instance ?: synchronized(this) {
                instance ?: Room
                    .databaseBuilder(
                        context.applicationContext,
                        LevelsDatabase::class.java,
                        "einkarcade.db",
                    ).allowMainThreadQueries()
                    .addMigrations(MIGRATION_1_2, MIGRATION_2_3, MIGRATION_3_4)
                    .build()
                    .also { instance = it }
            }
    }
}
