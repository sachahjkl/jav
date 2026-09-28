plugins {
    `java-library`
}

repositories {
    mavenCentral()
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of({{ java_version }}))
    }
}

dependencies {
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
}

val javConfiguration = providers.gradleProperty("jav.configuration").orElse("debug")

tasks.withType<JavaCompile>().configureEach {
    options.isDebug = javConfiguration.map { it != "release" }.get()
}

tasks.withType<AbstractArchiveTask>().configureEach {
    isPreserveFileTimestamps = false
    isReproducibleFileOrder = true
}
