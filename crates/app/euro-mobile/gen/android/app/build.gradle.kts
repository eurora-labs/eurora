import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

// Channel-aware identity. ANDROID_CHANNEL is set by scripts/release-android.sh
// and the publish.yaml workflow. Local builds default to the dev identity.
val androidChannel: String = (System.getenv("ANDROID_CHANNEL") ?: "dev").lowercase()

// Opt in to keeping unstripped Rust .so debug symbols inside the debug APK
// (needed only when attaching lldb to the running process). Default OFF — an
// unstripped libeuro_mobile.so is hundreds of MB and dominates emulator install
// time. Set KEEP_NATIVE_DEBUG_SYMBOLS=1 when you actually need native debugging.
val keepNativeDebugSymbols: Boolean =
    System.getenv("KEEP_NATIVE_DEBUG_SYMBOLS")?.let { it == "1" || it.equals("true", true) }
        ?: false
val channelApplicationId: String = when (androidChannel) {
    "release" -> "com.eurora_labs.eurora"
    "nightly" -> "com.eurora_labs.eurora.nightly"
    else -> "com.eurora_labs.eurora.dev"
}
val channelAppLabel: String = when (androidChannel) {
    "release" -> "Eurora"
    "nightly" -> "Eurora Nightly"
    else -> "Eurora Dev"
}

android {
    compileSdk = 36
    namespace = "com.eurora_labs.eurora.dev"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        manifestPlaceholders["tauriBrowserRedirectScheme"] = "eurora"
        manifestPlaceholders["appAuthRedirectScheme"] = "eurora"
        applicationId = channelApplicationId
        minSdk = 24
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
        resValue("string", "app_name", channelAppLabel)
        resValue("string", "main_activity_title", channelAppLabel)
    }
    signingConfigs {
        create("release") {
            val keystorePath = System.getenv("ANDROID_KEYSTORE_PATH")
            if (!keystorePath.isNullOrBlank()) {
                storeFile = file(keystorePath)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
            }
        }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = keepNativeDebugSymbols
            isMinifyEnabled = false
            if (keepNativeDebugSymbols) {
                packaging {
                    jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                    jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                    jniLibs.keepDebugSymbols.add("*/x86/*.so")
                    jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
                }
            }
        }
        getByName("release") {
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
            // Only attach the release signing config when the keystore env vars
            // are present. Without this guard, `pnpm tauri android build` from a
            // dev machine would fail at sign time.
            if (!System.getenv("ANDROID_KEYSTORE_PATH").isNullOrBlank()) {
                signingConfig = signingConfigs.getByName("release")
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        buildConfig = true
        // Tauri does not use any of these — turn them off so AGP doesn't
        // register their tasks for every variant.
        aidl = false
        renderScript = false
        shaders = false
        resValues = true
    }
    // Don't fail dev builds on lint findings. Release builds still get checked.
    lint {
        checkReleaseBuilds = true
        abortOnError = false
        ignoreWarnings = true
    }
}

// Skip wiring up Android/unit test variants for debug builds — we don't run
// instrumented tests from the Tauri Android dev loop, and registering them
// adds configuration overhead per ABI flavor.
androidComponents {
    beforeVariants(selector().withBuildType("debug")) { variant ->
        variant.enableAndroidTest = false
        variant.enableUnitTest = false
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")
