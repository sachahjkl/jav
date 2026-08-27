package {{ package_name }};

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class MainTest {
  @Test
  void greetingIncludesProjectName() {
    assertEquals("Hello from {{ project_name }}", Main.greeting());
  }
}
