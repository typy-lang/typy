// The Mandelbrot set of the Benchmarks Game, as the count of points of an N by N grid in it.
public final class Mandelbrot {
    public static void main(String[] args) {
        int n = Integer.parseInt(args[0]);
        long found = 0;
        for (int y = 0; y < n; y++) {
            double ci = 2.0 * y / n - 1.0;
            for (int x = 0; x < n; x++) {
                double cr = 2.0 * x / n - 1.5, zr = 0.0, zi = 0.0;
                boolean escaped = false;
                for (int i = 0; i < 50; i++) {
                    double nextR = zr * zr - zi * zi + cr;
                    zi = 2.0 * zr * zi + ci;
                    zr = nextR;
                    if (zr * zr + zi * zi > 4.0) {
                        escaped = true;
                        break;
                    }
                }
                if (!escaped) found++;
            }
        }
        System.out.println(found);
    }
}
