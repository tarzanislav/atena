import java.nio.file.*; import java.text.Normalizer; import java.util.*; import java.util.regex.*;
public class Bench {
  static final Pattern M = Pattern.compile("\\p{M}"), RX = Pattern.compile("[\\p{L}\\p{N}]+(?:[-'][\\p{L}\\p{N}]+)*");
  static String fold(String s) { return M.matcher(Normalizer.normalize(s.toLowerCase(Locale.ROOT).replace("ß", "ss"), Normalizer.Form.NFKD)).replaceAll(""); }
  static List<String> tokens(String s) { List<String> o = new ArrayList<>(); Matcher m = RX.matcher(fold(s)); while (m.find()) o.add(m.group()); return o; }
  record T(String s, boolean pre) {}
  static int match(List<String> t, List<T> ph) { int n = t.size(), m = ph.size(); if (m == 0 || n < m) return 0; int c = 0;
    for (int i = 0; i <= n - m; i++) { boolean ok = true; for (int j = 0; j < m; j++) { T k = ph.get(j); String x = t.get(i + j); if (k.pre ? !x.startsWith(k.s) : !x.equals(k.s)) { ok = false; break; } } if (ok) c++; } return c; }
  static String sub(String s, int a, int b) { int n = s.codePointCount(0, s.length()); a = Math.min(a, n); b = Math.min(b, n); return s.substring(s.offsetByCodePoints(0, a), s.offsetByCodePoints(0, b)); }
  public static void main(String[] a) throws Exception {
    Map<String, TreeMap<Integer, List<List<T>>>> prep = new HashMap<>();
    for (String line : Files.readAllLines(Path.of(a[1]))) { String[] p = line.split("\t"); List<T> ph = new ArrayList<>();
      for (String t : p[2].split(" ")) { int n = t.codePointCount(0, t.length()); ph.add(n >= 4 ? new T(n >= 7 ? sub(t, 0, n - 2) : t, true) : new T(t, false)); }
      prep.computeIfAbsent(p[0], k -> new TreeMap<>()).computeIfAbsent(Integer.parseInt(p[1]), k -> new ArrayList<>()).add(ph); }
    List<String[]> docs = new ArrayList<>(); for (String line : Files.readAllLines(Path.of(a[0]))) docs.add(line.split("\t", 3));
    long t0 = System.nanoTime(); int n = 0; double chk = 0;
    for (String[] d : docs) { List<String> tt = tokens(d[1]), ht = tokens(sub(d[2], 0, 1500)), rt = tokens(sub(d[2], 1500, 20000));
      for (List<List<T>> phrases : prep.getOrDefault(d[0], new TreeMap<>()).values()) { int w = 0; for (List<T> p : phrases) w += Math.min(3 * match(tt, p) + 2 * match(ht, p) + match(rt, p), 6);
        if (w > 0) { double s = 1 - Math.exp(-w / 3.0); if (s >= 0.45) { n++; chk += Math.round(s * 1000) / 1000.0; } } } }
    double dt = (System.nanoTime() - t0) / 1e9;
    System.out.printf("java (JVM, rovnaká trieda ako C#/.NET): %d docs in %.2f s = %.3f ms/doc; suggestions=%d checksum=%.3f%n", docs.size(), dt, dt / docs.size() * 1000, n, chk); } }
