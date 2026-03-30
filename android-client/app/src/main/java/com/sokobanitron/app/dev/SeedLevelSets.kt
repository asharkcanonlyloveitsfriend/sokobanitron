package com.sokobanitron.app.dev

import android.content.Context
import java.io.File

object SeedLevelSets {
    private const val ASSET_DIR = "level_sets"

    fun prepare(context: Context): File {
        val root = File(context.filesDir, "level_sets")
        val toImport = File(root, "to_import")
        val imported = File(root, "imported")
        val database = File(root, "sokobanitron.db")

        require(root.exists() || root.mkdirs()) { "Failed to create ${root.absolutePath}" }
        require(toImport.exists() || toImport.mkdirs()) {
            "Failed to create ${toImport.absolutePath}"
        }
        require(imported.exists() || imported.mkdirs()) {
            "Failed to create ${imported.absolutePath}"
        }

        if (database.exists() || toImport.listSlcFiles().isNotEmpty() || imported.listSlcFiles().isNotEmpty()) {
            return root
        }

        val assetNames = context.assets.list(ASSET_DIR).orEmpty().filter { name ->
            name.endsWith(".slc", ignoreCase = true)
        }.sorted()
        check(assetNames.isNotEmpty()) { "No bundled level sets found in assets/$ASSET_DIR." }

        for (assetName in assetNames) {
            val destination = File(toImport, assetName)
            context.assets.open("$ASSET_DIR/$assetName").use { input ->
                destination.outputStream().use { output ->
                    input.copyTo(output)
                }
            }
        }

        return root
    }

    private fun File.listSlcFiles(): List<File> =
        listFiles()?.filter { file ->
            file.isFile && file.extension.equals("slc", ignoreCase = true)
        }.orEmpty()
}
