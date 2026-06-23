plugins {
    application
}

repositories {
    mavenCentral()
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of({{ java_version }}))
    }
}

val javConfiguration = providers.gradleProperty("jav.configuration").orElse("debug")

dependencies {
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
}

tasks.test {
    useJUnitPlatform()
}

application {
    mainClass.set("{{ package_name }}.{{ main_class }}")
}

tasks.withType<JavaCompile>().configureEach {
    options.isDebug = javConfiguration.map { it != "release" }.get()
}
