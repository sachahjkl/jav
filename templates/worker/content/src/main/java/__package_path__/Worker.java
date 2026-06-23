package {{ package_name }};

import java.time.Instant;

public final class Worker {
    public static void main(String[] args) {
        System.out.println(new Worker().tick());
    }

    public String tick() {
        return "{{ project_name }} worker tick at " + Instant.now();
    }
}
