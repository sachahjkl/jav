package {{ package_name }};

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class MainTest {
    @Test
    void greetingIncludesProjectName() {
        assertEquals("Hello from {{ project_name }}", Main.greeting());
    }
}
