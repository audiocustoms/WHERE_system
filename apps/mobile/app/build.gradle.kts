import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

/** Die gebaute Oberfläche aus `apps/web` — dieselbe Datei, die auch die Desktop-Hülle lädt. */
val webBuild = rootProject.file("../web/build")

android {
    namespace = "de.where.mobile"
    compileSdk = 36

    defaultConfig {
        applicationId = "de.where.mobile"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}

/**
 * Kopiert `apps/web/build` nach `assets/web` im APK. Die Oberfläche liegt nicht im
 * Quellbaum: sie entsteht beim Web-Build und wird hier eingesammelt. Das Ziel ist
 * bewusst `src/main/assets/web` — dort findet Android sie ohne Zusatzlogik.
 */
abstract class SyncWebAssets : DefaultTask() {

    @get:InputDirectory
    abstract val quelle: DirectoryProperty

    @get:OutputDirectory
    abstract val ziel: DirectoryProperty

    @TaskAction
    fun kopieren() {
        val von = quelle.get().asFile
        val nach = ziel.get().asFile

        if (!von.resolve("index.html").isFile) {
            throw GradleException(
                "apps/web/build/index.html fehlt — zuerst die Oberfläche bauen: " +
                    "npm --prefix ../web run build"
            )
        }

        nach.deleteRecursively()
        von.walkTopDown()
            .filter { it.isFile && !it.name.endsWith(".map") }
            .forEach { datei ->
                // Android lässt Namen mit führendem `_` aus den Assets weg — SvelteKit
                // legt aber alles unter `_app`. Deshalb bekommt jedes solche Segment
                // ein `x` davor; `AppAssets` nimmt es beim Ausliefern wieder weg.
                val relativ = datei.relativeTo(von).path.replace('\\', '/')
                val ablage = nach.resolve(geschuetzt(relativ))
                ablage.parentFile?.mkdirs()
                datei.copyTo(ablage, overwrite = true)
            }
    }

    private fun geschuetzt(pfad: String): String =
        pfad.split('/').joinToString("/") { teil -> if (teil.startsWith("_")) "x$teil" else teil }
}

val syncWeb = tasks.register<SyncWebAssets>("syncWeb") {
    description = "Kopiert die gebaute Oberfläche aus apps/web in die Android-Assets."
    group = "build"
    quelle.set(webBuild)
    ziel.set(layout.projectDirectory.dir("src/main/assets/web"))
}

tasks.named("preBuild") {
    dependsOn(syncWeb)
}

tasks.named("clean") {
    doLast { delete(layout.projectDirectory.dir("src/main/assets/web")) }
}
