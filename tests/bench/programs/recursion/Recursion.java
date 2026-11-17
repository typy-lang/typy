// Recursive calls: the Fibonacci numbers by their definition.
public final class Recursion {
    static long fibonacci(long n) {
        return n < 2 ? n : fibonacci(n - 1) + fibonacci(n - 2);
    }

    public static void main(String[] args) {
        System.out.println(fibonacci(Long.parseLong(args[0])));
    }
}
