// The k-nucleotide of the Benchmarks Game, over a sequence of N drawn as fasta draws it.
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public final class KNucleotide {
    static final int IM = 139968, IA = 3877, IC = 29573;

    static String sequence(int n) {
        double[] homoSapiens = {0.3029549426680, 0.1979883004921, 0.1975473066391, 0.3015094502008};
        double[] table = new double[4];
        double total = 0.0;
        for (int i = 0; i < 4; i++) table[i] = total += homoSapiens[i];
        table[3] = 1.0;
        StringBuilder text = new StringBuilder(n);
        long seed = 42;
        for (int i = 0; i < n; i++) {
            seed = (seed * IA + IC) % IM;
            double drawn = 1.0 * seed / IM;
            int k = 0;
            while (drawn >= table[k]) k++;
            text.append("ACGT".charAt(k));
        }
        return text.toString();
    }

    static Map<String, Integer> counts(String text, int k) {
        Map<String, Integer> found = new HashMap<>();
        for (int i = 0; i + k <= text.length(); i++) found.merge(text.substring(i, i + k), 1, Integer::sum);
        return found;
    }

    public static void main(String[] args) {
        String text = sequence(Integer.parseInt(args[0]));
        StringBuilder out = new StringBuilder();
        for (int k = 1; k <= 2; k++) {
            List<Map.Entry<String, Integer>> ranked = new ArrayList<>(counts(text, k).entrySet());
            ranked.sort((a, b) -> !a.getValue().equals(b.getValue())
                    ? Integer.compare(b.getValue(), a.getValue())
                    : a.getKey().compareTo(b.getKey()));
            double total = text.length() - k + 1;
            for (Map.Entry<String, Integer> entry : ranked)
                out.append(String.format("%s %.3f%n", entry.getKey(), 100.0 * entry.getValue() / total));
            out.append('\n');
        }
        for (String fragment : new String[] {"GGT", "GGTA", "GGTATT", "GGTATTTTAATT", "GGTATTTTAATTTATAGT"})
            out.append(counts(text, fragment.length()).getOrDefault(fragment, 0)).append('\t').append(fragment).append('\n');
        System.out.print(out);
    }
}
