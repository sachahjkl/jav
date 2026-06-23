plugins {
    id("org.springframework.boot") version "{{ spring_boot_version }}"
    id("io.spring.dependency-management") version "1.1.7"
    application
    java
}

group = "{{ package_name }}"
version = "0.1.0-SNAPSHOT"

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
    {% for dependency in spring_gradle_dependencies %}
    implementation("{{ dependency }}")
    {% endfor %}
    {% if spring_has_lombok %}
    compileOnly("org.projectlombok:lombok")
    annotationProcessor("org.projectlombok:lombok")
    {% endif %}
    {% for dependency in spring_runtime_gradle_dependencies %}
    runtimeOnly("{{ dependency }}")
    {% endfor %}
    testImplementation("org.springframework.boot:spring-boot-starter-test")
}

tasks.test {
    useJUnitPlatform()
}

application {
    mainClass.set("{{ package_name }}.Application")
}

tasks.withType<JavaCompile>().configureEach {
    options.isDebug = javConfiguration.map { it != "release" }.get()
}
