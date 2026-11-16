// A small interpreter: arithmetic expressions written out, tokenized and evaluated.
import java.util.ArrayList;
import java.util.List;

public final class Parser {
    static final long MODULUS = 1000003;
    final List<String> items;
    int position = 0;

    Parser(List<String> items) {
        this.items = items;
    }

    static String expression(long seed) {
        String[] parts = new String[12];
        for (int k = 0; k < 12; k++) {
            String value = Long.toString((seed * 31 + k * 17) % 97 + 1);
            parts[k] = k % 4 == 0 ? "(" + value : k % 4 == 3 ? value + ")" : value;
        }
        String[] operators = {"+", "*", "-", "+"};
        StringBuilder text = new StringBuilder(parts[0]);
        for (int k = 1; k < 12; k++)
            text.append(' ').append(operators[(int) ((seed + k) % 4)]).append(' ').append(parts[k]);
        return text.toString();
    }

    static List<String> tokens(String text) {
        List<String> found = new ArrayList<>();
        StringBuilder number = new StringBuilder();
        for (char c : text.toCharArray()) {
            if (Character.isDigit(c)) {
                number.append(c);
                continue;
            }
            if (number.length() > 0) {
                found.add(number.toString());
                number.setLength(0);
            }
            if (c != ' ') found.add(String.valueOf(c));
        }
        if (number.length() > 0) found.add(number.toString());
        return found;
    }

    String peek() {
        return position < items.size() ? items.get(position) : "";
    }

    String take() {
        return items.get(position++);
    }

    long sum() {
        long value = product();
        while (peek().equals("+") || peek().equals("-")) {
            if (take().equals("+")) value = Math.floorMod(value + product(), MODULUS);
            else value = Math.floorMod(value - product(), MODULUS);
        }
        return value;
    }

    long product() {
        long value = atom();
        while (peek().equals("*")) {
            take();
            value = Math.floorMod(value * atom(), MODULUS);
        }
        return value;
    }

    long atom() {
        String item = take();
        if (item.equals("(")) {
            long value = sum();
            take();
            return value;
        }
        return Long.parseLong(item);
    }

    public static void main(String[] args) {
        int n = Integer.parseInt(args[0]);
        long total = 0;
        for (int i = 0; i < n; i++) total = Math.floorMod(total + new Parser(tokens(expression(i))).sum(), MODULUS);
        System.out.println(total);
    }
}
