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

dependencies {
    {% if spring_has_web %}
    implementation("org.springframework.boot:spring-boot-starter-web")
    {% endif %}
    {% if spring_has_actuator %}
    implementation("org.springframework.boot:spring-boot-starter-actuator")
    {% endif %}
    {% if spring_has_data_jpa %}
    implementation("org.springframework.boot:spring-boot-starter-data-jpa")
    {% endif %}
    {% if spring_has_security %}
    implementation("org.springframework.boot:spring-boot-starter-security")
    {% endif %}
    {% if spring_has_lombok %}
    compileOnly("org.projectlombok:lombok")
    annotationProcessor("org.projectlombok:lombok")
    {% endif %}
    {% if spring_has_postgresql %}
    runtimeOnly("org.postgresql:postgresql")
    {% endif %}
    testImplementation("org.springframework.boot:spring-boot-starter-test")
}

tasks.test {
    useJUnitPlatform()
}

application {
    mainClass.set("{{ package_name }}.Application")
}
