// String work as programs do it: joining, splitting, case, searching and replacing.
public final class Strings {
    static final String[] WORDS = {"alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"};

    static String stripped(String line, char c) {
        int start = 0, end = line.length();
        while (start < end && line.charAt(start) == c) start++;
        while (end > start && line.charAt(end - 1) == c) end--;
        return line.substring(start, end);
    }

    public static void main(String[] args) {
        int n = Integer.parseInt(args[0]);
        long total = 0;
        for (int i = 0; i < n; i++) {
            String[] parts = new String[8];
            for (int k = 0; k < 8; k++) parts[k] = WORDS[(i + k) % WORDS.length];
            String line = String.join(" ", parts);
            String upper = line.toUpperCase(java.util.Locale.ROOT);
            String[] pieces = upper.split(" ", -1);
            long a = line.chars().filter(c -> c == 'a').count();
            total += pieces.length + upper.indexOf("GAMMA") + a;
            total += line.replace("beta", "b").length() + stripped(line, 'a').length();
            if (line.startsWith("alpha")) total++;
        }
        System.out.println(total);
    }
}
