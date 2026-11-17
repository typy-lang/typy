// The spectral norm of the Benchmarks Game: the largest eigenvalue of an infinite matrix.
public final class SpectralNorm {
    static double element(int i, int j) {
        return 1.0 / ((long) (i + j) * (i + j + 1) / 2 + i + 1);
    }

    static void times(double[] u, double[] v) {
        for (int i = 0; i < v.length; i++) {
            double total = 0.0;
            for (int j = 0; j < u.length; j++) total += element(i, j) * u[j];
            v[i] = total;
        }
    }

    static void timesTransposed(double[] u, double[] v) {
        for (int i = 0; i < v.length; i++) {
            double total = 0.0;
            for (int j = 0; j < u.length; j++) total += element(j, i) * u[j];
            v[i] = total;
        }
    }

    public static void main(String[] args) {
        int n = Integer.parseInt(args[0]);
        double[] u = new double[n], v = new double[n], scratch = new double[n];
        java.util.Arrays.fill(u, 1.0);
        for (int k = 0; k < 10; k++) {
            times(u, scratch);
            timesTransposed(scratch, v);
            times(v, scratch);
            timesTransposed(scratch, u);
        }
        double vbv = 0.0, vv = 0.0;
        for (int i = 0; i < n; i++) {
            vbv += u[i] * v[i];
            vv += v[i] * v[i];
        }
        System.out.printf("%.9f%n", Math.sqrt(vbv / vv));
    }
}
