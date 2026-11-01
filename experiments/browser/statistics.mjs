export function summary(samples) {
  if (samples.some(value => !Number.isFinite(value) || value < 0)) throw new TypeError("Invalid duration sample");
  const sorted = [...samples].sort((a, b) => a - b);
  const percentile = fraction => sorted.length ? sorted[Math.ceil(sorted.length * fraction) - 1] : null;
  return { count: sorted.length, p50: percentile(.5), p95: percentile(.95), p99: percentile(.99),
    max: sorted.at(-1) ?? null, over33ms: sorted.filter(value => value > 1000 / 30).length,
    over50ms: sorted.filter(value => value > 50).length };
}
