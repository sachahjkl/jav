package {{ package_name }};

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class LibraryTest {
    @Test
    void exposesLibraryName() {
        assertEquals("{{ project_name }}", new Library().name());
    }
}
