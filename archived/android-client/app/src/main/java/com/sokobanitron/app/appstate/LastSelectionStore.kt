package com.sokobanitron.app.appstate

import android.content.Context
import android.content.SharedPreferences
import androidx.core.content.edit

class LastSelectionStore(
    context: Context,
) {
    companion object {
        private const val PREFS_NAME = "eink_arcade_prefs"
        private const val KEY_SET_ID = "current_set_id"
        private const val KEY_PUZZLE_ID = "current_puzzle_id"
    }

    private val prefs: SharedPreferences =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun save(
        setId: Int,
        puzzleId: Int,
    ) {
        prefs.edit {
            putInt(KEY_SET_ID, setId)
                .putInt(KEY_PUZZLE_ID, puzzleId)
        }
    }

    fun load(): Pair<Int, Int> {
        val savedSetId = prefs.getInt(KEY_SET_ID, 0)
        val savedPuzzleId = prefs.getInt(KEY_PUZZLE_ID, 0)
        return savedSetId to savedPuzzleId
    }
}
