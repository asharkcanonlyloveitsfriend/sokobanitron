package com.example.einkarcade

import android.content.Context
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.ui.Modifier
import com.example.einkarcade.ui.screens.GameScreen
import com.example.einkarcade.ui.theme.EinkArcadeTheme

class MainActivity : ComponentActivity() {
    companion object {
        // Optional factory for injecting a custom GameController (tests)
        var gameControllerFactory: ((Context) -> GameController)? = null
    }

    private lateinit var gameController: GameController

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        gameController = (gameControllerFactory?.invoke(this)) ?: GameController(this, null)
        enableEdgeToEdge()
        setContent {
            EinkArcadeTheme {
                Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                    GameScreen(
                        modifier = Modifier.padding(innerPadding),
                        gameController = gameController,
                    )
                }
            }
        }
    }
}
