package {{ package_name }};

import java.util.Arrays;

public final class CommandApp {
    private CommandApp() {
    }

    public static void main(String[] args) {
        System.out.println(handle(args));
    }

    public static String handle(String[] args) {
        if (args.length == 0 || Arrays.asList(args).contains("--help")) {
            return "Usage: {{ project_name }} <command> [options]";
        }

        return "command=" + args[0];
    }
}
