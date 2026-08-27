package {{ package_name }};

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class LibraryTest {
  @Test
  void exposesLibraryName() {
    assertEquals("{{ project_name }}", new Library().name());
  }
}
