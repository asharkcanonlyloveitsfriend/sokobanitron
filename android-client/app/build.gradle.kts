plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

val syncBundledLevelSets by tasks.registering(Sync::class) {
    from(layout.projectDirectory.dir("../../tmp/level_sets/to_import")) {
        include("*.slc")
    }
    into(layout.buildDirectory.dir("generated/assets/bundledLevelSets/level_sets"))
}

android {
    namespace = "com.sokobanitron.app.dev"
    compileSdk {
        version = release(36)
    }

    defaultConfig {
        applicationId = "com.sokobanitron.app.dev"
        minSdk = 30
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
    kotlinOptions {
        jvmTarget = "11"
    }

    sourceSets.named("main") {
        assets.srcDir(layout.buildDirectory.dir("generated/assets/bundledLevelSets"))
    }
}

tasks.named("preBuild").configure {
    dependsOn(syncBundledLevelSets)
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)
    implementation(libs.material)
    implementation(libs.androidx.activity)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
}
