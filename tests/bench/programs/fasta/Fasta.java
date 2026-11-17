// The fasta of the Benchmarks Game: DNA sequences, repeated and drawn from a generator.
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.OutputStreamWriter;

public final class Fasta {
    static final int IM = 139968, IA = 3877, IC = 29573, WIDTH = 60;
    static final String ALU =
            "GGCCGGGCGCGGTGGCTCACGCCTGTAATCCCAGCACTTTGGGAGGCCGAGGCGGGCGGATCACCTGAGGTCAGGAGTTCGAG"
            + "ACCAGCCTGGCCAACATGGTGAAACCCCGTCTCTACTAAAAATACAAAAATTAGCCGGGCGTGGTGGCGCGCGCCTGTAATCCC"
            + "AGCTACTCGGGAGGCTGAGGCAGGAGAATCGCTTGAACCCGGGAGGCGGAGGTTGCAGTGAGCCGAGATCGCGCCACTGCACTCC"
            + "AGCCTGGGCGACAGAGCGAGACTCCGTCTCAAAAA";
    static long seed = 42;

    static double draw(double maximum) {
        seed = (seed * IA + IC) % IM;
        return maximum * seed / IM;
    }

    static void repeatFasta(BufferedWriter out, String header, String source, int n)
            throws IOException {
        out.write(header);
        out.write('\n');
        String extended = source + source.substring(0, WIDTH);
        int position = 0;
        while (n > 0) {
            int line = Math.min(WIDTH, n);
            out.write(extended, position, line);
            out.write('\n');
            position = (position + line) % source.length();
            n -= line;
        }
    }

    static void randomFasta(BufferedWriter out, String header, String symbols,
            double[] probabilities, int n) throws IOException {
        out.write(header);
        out.write('\n');
        double[] table = new double[probabilities.length];
        double total = 0.0;
        for (int i = 0; i < probabilities.length; i++) table[i] = total += probabilities[i];
        table[table.length - 1] = 1.0;
        char[] line = new char[WIDTH];
        while (n > 0) {
            int size = Math.min(WIDTH, n);
            for (int i = 0; i < size; i++) {
                double drawn = draw(1.0);
                int k = 0;
                while (drawn >= table[k]) k++;
                line[i] = symbols.charAt(k);
            }
            out.write(line, 0, size);
            out.write('\n');
            n -= size;
        }
    }

    public static void main(String[] args) throws IOException {
        int n = Integer.parseInt(args[0]);
        try (BufferedWriter out = new BufferedWriter(new OutputStreamWriter(System.out), 65536)) {
            repeatFasta(out, ">ONE Homo sapiens alu", ALU, 2 * n);
            randomFasta(out, ">TWO IUB ambiguity codes", "acgtBDHKMNRSVWY",
                    new double[] {0.27, 0.12, 0.12, 0.27, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02, 0.02,
                        0.02, 0.02, 0.02, 0.02},
                    3 * n);
            randomFasta(out, ">THREE Homo sapiens frequency", "acgt",
                    new double[] {0.3029549426680, 0.1979883004921, 0.1975473066391,
                        0.3015094502008},
                    5 * n);
        }
    }
}
