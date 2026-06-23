package {{ package_name }};

import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class WorkerTest {
    @Test
    void tickIncludesProjectName() {
        assertTrue(new Worker().tick().startsWith("{{ project_name }} worker tick"));
    }
}
