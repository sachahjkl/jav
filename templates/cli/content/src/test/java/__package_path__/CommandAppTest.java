package {{ package_name }};

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class CommandAppTest {
    @Test
    void rendersHelpWhenNoCommandIsProvided() {
        assertEquals("Usage: {{ project_name }} <command> [options]", CommandApp.handle(new String[0]));
    }
}
