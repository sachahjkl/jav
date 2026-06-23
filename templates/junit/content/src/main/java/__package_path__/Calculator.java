package {{ package_name }};

public final class Calculator {
    public static void main(String[] args) {
        System.out.println(new Calculator().add(2, 2));
    }

    public int add(int left, int right) {
        return left + right;
    }
}
