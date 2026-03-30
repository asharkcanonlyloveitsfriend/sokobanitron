package com.sokobanitron.app.dev

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {
    private lateinit var gameSurface: RustSurfaceView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        gameSurface = findViewById(R.id.game_surface)
    }

    override fun onDestroy() {
        if (::gameSurface.isInitialized) {
            gameSurface.release()
        }
        super.onDestroy()
    }
}
