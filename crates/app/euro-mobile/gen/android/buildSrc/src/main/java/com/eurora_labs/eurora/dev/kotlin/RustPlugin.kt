import com.android.build.api.dsl.ApplicationExtension
import java.util.Properties
import org.gradle.api.DefaultTask
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.kotlin.dsl.configure
import org.gradle.kotlin.dsl.get

const val TASK_GROUP = "rust"

open class Config {
    lateinit var rootDirRel: String
}

/// Resolve a build property by checking, in order: Gradle's `-P`/`gradle.properties`
/// lookup, then the project's `local.properties` (gitignored, per-developer).
/// Lets a single dev limit ABIs to their host arch without committing global
/// gradle.properties changes that would also constrain CI release builds.
private fun Project.findBuildProperty(name: String): String? {
    findProperty(name)?.let { return it as? String }
    val localPropsFile = rootProject.file("local.properties")
    if (!localPropsFile.exists()) return null
    val props = Properties()
    localPropsFile.inputStream().use { props.load(it) }
    return props.getProperty(name)
}

open class RustPlugin : Plugin<Project> {
    private lateinit var config: Config

    override fun apply(project: Project) = with(project) {
        config = extensions.create("rust", Config::class.java)

        val defaultAbiList = listOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64");
        val abiList = findBuildProperty("abiList")?.split(',') ?: defaultAbiList

        val defaultArchList = listOf("arm64", "arm", "x86", "x86_64");
        val archList = findBuildProperty("archList")?.split(',') ?: defaultArchList

        val targetsList = findBuildProperty("targetList")?.split(',') ?: listOf("aarch64", "armv7", "i686", "x86_64")

        extensions.configure<ApplicationExtension> {
            @Suppress("UnstableApiUsage")
            flavorDimensions.add("abi")
            productFlavors {
                create("universal") {
                    dimension = "abi"
                    ndk {
                        abiFilters += abiList
                    }
                }
                defaultArchList.forEachIndexed { index, arch ->
                    create(arch) {
                        dimension = "abi"
                        ndk {
                            abiFilters.add(defaultAbiList[index])
                        }
                    }
                }
            }
        }

        afterEvaluate {
            for (profile in listOf("debug", "release")) {
                val profileCapitalized = profile.replaceFirstChar { it.uppercase() }
                val buildTask = tasks.maybeCreate(
                    "rustBuildUniversal$profileCapitalized",
                    DefaultTask::class.java
                ).apply {
                    group = TASK_GROUP
                    description = "Build dynamic library in $profile mode for all targets"
                }

                tasks["mergeUniversal${profileCapitalized}JniLibFolders"].dependsOn(buildTask)

                for (targetPair in targetsList.withIndex()) {
                    val targetName = targetPair.value
                    val targetArch = archList[targetPair.index]
                    val targetArchCapitalized = targetArch.replaceFirstChar { it.uppercase() }
                    val targetBuildTask = project.tasks.maybeCreate(
                        "rustBuild$targetArchCapitalized$profileCapitalized",
                        BuildTask::class.java
                    ).apply {
                        group = TASK_GROUP
                        description = "Build dynamic library in $profile mode for $targetArch"
                        rootDirRel = config.rootDirRel
                        target = targetName
                        release = profile == "release"
                        projectDir = project.projectDir
                    }

                    buildTask.dependsOn(targetBuildTask)
                    tasks["merge$targetArchCapitalized${profileCapitalized}JniLibFolders"].dependsOn(
                        targetBuildTask
                    )
                }
            }
        }
    }
}