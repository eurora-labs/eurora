// Toolchain floor:
//   * Android Gradle Plugin 8.6+   (required for compileSdk 36)
//   * Kotlin 2.1+                  (matches `settings.gradle` plugin pin)
//   * JDK 17                       (sourceCompatibility / jvmTarget below)
// Host Tauri apps pinning older AGP versions must upgrade before consuming
// this library.
plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "app.tauri.appauth"
    compileSdk = 36

    defaultConfig {
        minSdk = 24

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")

        // Default substitutions for the redirect-scheme placeholders used by
        // this library's `BrowserSessionActivity` and the AppAuth-Android
        // `RedirectUriReceiverActivity` we merge in. Host apps override
        // these via their own `manifestPlaceholders[...]`; the defaults let
        // the library's own manifest-merge step (and the Robolectric unit
        // tests) succeed without consumer config.
        manifestPlaceholders["tauriBrowserRedirectScheme"] = "tauri.appauth.test"
        manifestPlaceholders["appAuthRedirectScheme"] = "tauri.appauth.test"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    testOptions {
        // Robolectric loads merged AndroidManifest.xml + resources from the
        // library's own outputs; without this flag the runtime starts with an
        // empty package and Activity lookup fails.
        unitTests.isIncludeAndroidResources = true
    }
}

// Pin the unit-test JVM to JDK 17. Robolectric 4.14's bundled ASM throws
// `IllegalArgumentException` on class files compiled for newer JVMs (the
// Gradle daemon defaults to whatever JDK launched it, often JDK 21+), so
// we route the test task through Gradle's toolchain resolver instead.
val javaToolchains = extensions.getByType<JavaToolchainService>()
tasks.withType<Test>().configureEach {
    javaLauncher.set(javaToolchains.launcherFor {
        languageVersion.set(JavaLanguageVersion.of(17))
    })
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.9.3")
    implementation("androidx.browser:browser:1.8.0")
    implementation("net.openid:appauth:0.11.1")
    // Annotations carry no executable code, and the host Tauri runtime
    // brings a matching `jackson-annotations` in transitively via
    // `jackson-databind`. `compileOnly` avoids version skew with whatever
    // the host pins. Databind itself stays `implementation` because
    // `AuthEvent.Serializer` subclasses `JsonSerializer`, which must
    // resolve at runtime when this library is built in isolation.
    compileOnly("com.fasterxml.jackson.core:jackson-annotations:2.15.3")
    implementation("com.fasterxml.jackson.core:jackson-databind:2.15.3")
    implementation(project(":tauri-android"))

    // Local unit tests run on the JVM; Robolectric supplies the Android
    // runtime (resources, manifest merging, `Looper`, `org.json`) so we can
    // exercise `Activity` lifecycles and `Invoke.reject(...)` without a
    // device. AndroidX `core-ktx` is needed transitively for
    // `ActivityScenario`.
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.robolectric:robolectric:4.14.1")
    testImplementation("androidx.test:core-ktx:1.6.1")
    testImplementation("androidx.test.ext:junit-ktx:1.2.1")
}
