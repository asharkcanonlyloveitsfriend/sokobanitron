package com.example.einkarcade.data

import android.content.Context
import com.example.einkarcade.content.LevelSet
import com.example.einkarcade.data.db.LevelEntity
import com.example.einkarcade.data.db.LevelSetEntity
import com.example.einkarcade.data.db.LevelsDatabase
import com.example.einkarcade.data.db.PuzzleEntity
import com.example.einkarcade.sokoban.Level
import com.example.einkarcade.sokoban.Position
import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.net.HttpURLConnection
import java.net.URL
import java.time.Instant
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.util.concurrent.Executors

// Repository for loading/saving level sets.
class LevelsRepository(
    private val context: Context,
) {
    companion object {
        private const val DEFAULT_SYNC_ENDPOINT = "http://192.168.0.75:8000/api/sync"
    }

    private val database = LevelsDatabase.getInstance(context)
    private val dao = database.levelsDao()
    private val utcFormatter =
        DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss").withZone(ZoneOffset.UTC)

    fun loadSets(): List<LevelSet>? {
        if (dao.countLevelSets() == 0) {
            bootstrapFromServer()
        }
        val sets = dao.getAllLevelSetsWithLevels()
        if (sets.isEmpty()) return null
        return sets.map { set ->
            val levels =
                set.levels.sortedBy { it.level.id }.map { levelWithPuzzle ->
                    val level =
                        Level.fromAscii(
                            levelWithPuzzle.level.title,
                            levelWithPuzzle.puzzle.grid,
                            levelWithPuzzle.level.puzzleId,
                        )
                    level.setRating(levelWithPuzzle.puzzle.rating)
                    level.setStarred(levelWithPuzzle.puzzle.isStarred)
                    level.setCompletedAt(levelWithPuzzle.puzzle.lastCompletedAt)
                    level
                }
            LevelSet(
                id = set.levelSet.id,
                name = set.levelSet.title,
                levels = levels,
            )
        }
    }

    fun updateRating(level: Level) {
        dao.updatePuzzleRating(level.puzzleId, level.rating)
    }

    fun updateStarred(level: Level) {
        dao.updatePuzzleStarred(level.puzzleId, level.isStarred)
    }

    fun recordCompletion(
        level: Level,
        solutionHistory: List<List<com.example.einkarcade.sokoban.Position>>,
    ): String {
        val normalized = normalizeSolution(solutionHistory)
        val newPushCount = normalized.size
        val existingSolutionJson = dao.getUserSolution(level.puzzleId)
        val timestamp = utcFormatter.format(Instant.now())

        val shouldPersistSolution =
            if (existingSolutionJson == null) {
                true
            } else {
                val existingPushCount =
                    try {
                        JSONArray(existingSolutionJson).length()
                    } catch (e: Exception) {
                        // If parsing fails, treat as no existing solution
                        Int.MAX_VALUE
                    }
                newPushCount < existingPushCount
            }

        if (shouldPersistSolution) {
            val userSolutionJson =
                if (normalized.isEmpty()) {
                    null
                } else {
                    val outerArray = JSONArray()
                    for (path in normalized) {
                        val pathArray = JSONArray()
                        for (pos in path) {
                            val posArray = JSONArray()
                            posArray.put(pos.row)
                            posArray.put(pos.col)
                            pathArray.put(posArray)
                        }
                        outerArray.put(pathArray)
                    }
                    outerArray.toString()
                }
            dao.updatePuzzleCompletion(level.puzzleId, timestamp, userSolutionJson)
        } else {
            dao.updatePuzzleCompletion(level.puzzleId, timestamp, existingSolutionJson)
        }

        return timestamp
    }

    private fun normalizeSolution(history: List<List<Position>>): List<List<Position>> {
        if (history.isEmpty()) return history
        val result = mutableListOf<List<Position>>()
        for (path in history) {
            if (result.isEmpty()) {
                result.add(path)
            } else {
                val last = result.last()
                if (last.last() == path.first()) {
                    val merged = last + path.drop(1)
                    result[result.lastIndex] = merged
                } else {
                    result.add(path)
                }
            }
        }
        return result
    }

    fun syncWithServer(endpoint: String = DEFAULT_SYNC_ENDPOINT) {
        val requestJson = buildSyncRequestJson(dao.getPuzzlesForSync())
        val responseJson = postJson(endpoint, requestJson)
        val response = parseSyncResponse(responseJson)
        database.runInTransaction {
            dao.clearLevels()
            dao.clearLevelSets()
            dao.clearPuzzles()
            dao.insertLevelSets(response.levelSets)
            dao.insertPuzzles(response.puzzles)
            dao.insertLevels(response.levels)
        }
    }

    private fun buildSyncRequestJson(puzzles: List<PuzzleEntity>): String {
        val puzzleArray = JSONArray()
        for (puzzle in puzzles) {
            val puzzleJson = JSONObject()
            puzzleJson.put("puzzle_id", puzzle.id)
            puzzleJson.put("rating", puzzle.rating)
            puzzleJson.put("is_starred", puzzle.isStarred)
            if (puzzle.lastCompletedAt == null) {
                puzzleJson.put("last_completed_at", JSONObject.NULL)
            } else {
                puzzleJson.put("last_completed_at", puzzle.lastCompletedAt)
            }
            if (puzzle.userSolution == null) {
                puzzleJson.put("user_solution", JSONObject.NULL)
            } else {
                puzzleJson.put("user_solution", puzzle.userSolution)
            }
            puzzleArray.put(puzzleJson)
        }
        val root = JSONObject()
        root.put("puzzles", puzzleArray)
        return root.toString()
    }

    private data class SyncResponseData(
        val levelSets: List<LevelSetEntity>,
        val levels: List<LevelEntity>,
        val puzzles: List<PuzzleEntity>,
    )

    @Throws(JSONException::class)
    private fun parseSyncResponse(jsonText: String): SyncResponseData {
        val root = JSONObject(jsonText)
        val levelSetsJson = root.getJSONArray("level_sets")
        val levelsJson = root.getJSONArray("levels")
        val puzzlesJson = root.getJSONArray("puzzles")

        val levelSets = ArrayList<LevelSetEntity>(levelSetsJson.length())
        for (i in 0 until levelSetsJson.length()) {
            val item = levelSetsJson.getJSONObject(i)
            levelSets.add(
                LevelSetEntity(
                    id = item.getInt("id"),
                    title = item.getString("title"),
                ),
            )
        }

        val puzzles = ArrayList<PuzzleEntity>(puzzlesJson.length())
        for (i in 0 until puzzlesJson.length()) {
            val item = puzzlesJson.getJSONObject(i)
            val lastCompletedAt =
                if (item.isNull("last_completed_at")) {
                    null
                } else {
                    item.getString("last_completed_at")
                }
            val userSolution =
                if (item.isNull("user_solution")) {
                    null
                } else {
                    item.getString("user_solution")
                }
            puzzles.add(
                PuzzleEntity(
                    id = item.getInt("id"),
                    grid = item.getString("grid"),
                    rating = item.getInt("rating"),
                    isStarred = parseBooleanField(item, "is_starred"),
                    lastCompletedAt = lastCompletedAt,
                    userSolution = userSolution,
                ),
            )
        }

        val levels = ArrayList<LevelEntity>(levelsJson.length())
        for (i in 0 until levelsJson.length()) {
            val item = levelsJson.getJSONObject(i)
            levels.add(
                LevelEntity(
                    id = item.getInt("id"),
                    title = item.getString("title"),
                    levelSetId = item.getInt("level_set_id"),
                    puzzleId = item.getInt("puzzle_id"),
                ),
            )
        }
        return SyncResponseData(levelSets = levelSets, levels = levels, puzzles = puzzles)
    }

    private fun parseBooleanField(
        item: JSONObject,
        key: String,
    ): Boolean {
        val raw = item.opt(key)
        return when (raw) {
            is Boolean -> raw
            is Number -> raw.toInt() != 0
            is String -> raw.equals("true", ignoreCase = true) || raw == "1"
            else -> false
        }
    }

    @Throws(Exception::class)
    private fun postJson(
        endpoint: String,
        body: String,
    ): String {
        val url = URL(endpoint)
        val connection =
            (url.openConnection() as HttpURLConnection).apply {
                requestMethod = "POST"
                connectTimeout = 10_000
                readTimeout = 15_000
                doInput = true
                doOutput = true
                setRequestProperty("Content-Type", "application/json")
            }
        connection.outputStream.use { stream ->
            stream.write(body.toByteArray(Charsets.UTF_8))
        }
        val responseCode = connection.responseCode
        val stream =
            if (responseCode in 200..299) {
                connection.inputStream
            } else {
                connection.errorStream
            }
        val response = BufferedReader(InputStreamReader(stream)).use { it.readText() }
        connection.disconnect()
        if (responseCode !in 200..299) {
            throw RuntimeException("Sync failed ($responseCode): $response")
        }
        return response
    }

    private fun bootstrapFromServer() {
        val executor = Executors.newSingleThreadExecutor()
        try {
            val future = executor.submit { syncWithServer() }
            future.get()
        } catch (e: Exception) {
            throw IllegalStateException(
                "BootstrapFailed: server unreachable or sync error. Is the server running?",
                e,
            )
        } finally {
            executor.shutdown()
        }
        if (dao.countLevelSets() == 0) {
            throw IllegalStateException("BootstrapFailed: server returned no level sets")
        }
    }
}
