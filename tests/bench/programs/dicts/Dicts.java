// Dictionary work as programs do it: counting words, then ranking the most frequent.
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public final class Dicts {
    public static void main(String[] args) {
        int n = Integer.parseInt(args[0]);
        Map<String, Integer> found = new HashMap<>();
        long seed = 1;
        for (int i = 0; i < n; i++) {
            seed = (seed * 1103515245L + 12345) % 2147483648L;
            found.merge("w" + (seed % 5000), 1, Integer::sum);
        }
        List<Map.Entry<String, Integer>> ranked = new ArrayList<>(found.entrySet());
        ranked.sort((a, b) -> !a.getValue().equals(b.getValue())
                ? Integer.compare(b.getValue(), a.getValue())
                : a.getKey().compareTo(b.getKey()));
        System.out.println(found.size());
        for (int i = 0; i < 5 && i < ranked.size(); i++)
            System.out.println(ranked.get(i).getKey() + " " + ranked.get(i).getValue());
    }
}
