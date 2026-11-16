// The fannkuch-redux of the Benchmarks Game: flips of every permutation of N items.
public final class FannkuchRedux {
    public static void main(String[] args) {
        int n = Integer.parseInt(args[0]);
        int[] first = new int[n], perm = new int[n], count = new int[n];
        for (int i = 0; i < n; i++) first[i] = i;
        long maxFlips = 0, checksum = 0, permutations = 0;
        int r = n;
        for (;;) {
            for (; r != 1; r--) count[r - 1] = r;
            System.arraycopy(first, 0, perm, 0, n);
            long flips = 0;
            for (int k = perm[0]; k != 0; k = perm[0]) {
                for (int i = 0, j = k; i < j; i++, j--) {
                    int held = perm[i];
                    perm[i] = perm[j];
                    perm[j] = held;
                }
                flips++;
            }
            maxFlips = Math.max(maxFlips, flips);
            checksum += permutations % 2 == 0 ? flips : -flips;
            for (;;) {
                if (r == n) {
                    System.out.println(checksum);
                    System.out.println("Pfannkuchen(" + n + ") = " + maxFlips);
                    return;
                }
                int head = first[0];
                System.arraycopy(first, 1, first, 0, r);
                first[r] = head;
                if (--count[r] > 0) break;
                r++;
            }
            permutations++;
        }
    }
}
