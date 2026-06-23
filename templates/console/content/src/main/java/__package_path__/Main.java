package {{ package_name }};

public final class Main {
    private Main() {
    }

    public static void main(String[] args) {
        System.out.println(greeting());
    }

    public static String greeting() {
        return "Hello from {{ project_name }}";
    }
}
