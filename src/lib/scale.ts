// Network speed meter scale & geometry.
// Uses a 180° semicircle (180° left → 270° top → 360° right) with evenly spaced
// standard network landmarks [0, 1, 5, 10, 50, 100, 500, 1,000] Mbps (scaling
// cleanly to multi-Gbps when aggregated throughput exceeds 1,000 Mbps).

/** Scale max steps matching `metrics::sampler::SCALE_STEPS` on the Rust side. */
export const SCALE_STEPS = [
  10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000, 25000, 50000, 100000, 250000, 500000, 1000000,
] as const;

/** 180° semicircle geometry (0° = right, clockwise in screen coords). */
export const ARC_START_DEG = 180;
export const ARC_END_DEG = 360;
export const ARC_SWEEP_DEG = ARC_END_DEG - ARC_START_DEG;

/** Standard Google-style network speed markings up to 1 Gbps. */
export const DEFAULT_NETWORK_TICKS = [0, 1, 5, 10, 50, 100, 500, 1000] as const;

/** Returns 8 evenly-spaced network landmarks covering `scaleMax`.
 * Up to 1,000 Mbps, always shows `[0, 1, 5, 10, 50, 100, 500, 1000]`. */
export function tickValues(scaleMax: number): number[] {
  if (scaleMax <= 1000) return [...DEFAULT_NETWORK_TICKS];
  if (scaleMax <= 2500) return [0, 5, 10, 50, 100, 500, 1000, 2500];
  if (scaleMax <= 5000) return [0, 10, 50, 100, 500, 1000, 2500, 5000];
  if (scaleMax <= 10000) return [0, 10, 50, 100, 500, 1000, 5000, 10000];
  if (scaleMax <= 50000) return [0, 50, 100, 500, 1000, 5000, 10000, 50000];
  if (scaleMax <= 100000) return [0, 100, 500, 1000, 5000, 10000, 50000, 100000];
  return [0, 500, 1000, 5000, 10000, 50000, 100000, 1000000];
}

/** Formats landmark tick numbers cleanly (`1,000`, `2.5G`, `10G`, etc.). */
export function formatTickLabel(value: number): string {
  if (value === 0) return "0";
  if (value === 1000) return "1,000";
  if (value > 1000) {
    const gbps = value / 1000;
    return Number.isInteger(gbps) ? `${gbps}G` : `${gbps.toFixed(1)}G`;
  }
  return String(value);
}

/** Piecewise-linear mapping across the network landmarks so each printed tick
 * sits at an exact uniform angle along the semicircle and speeds align cleanly
 * with the markings. */
export function valueToFraction(value: number, scaleMax: number): number {
  const ticks = tickValues(scaleMax);
  const v = Math.max(0, value);
  const maxTick = ticks[ticks.length - 1];
  if (v <= 0 || maxTick <= 0) return 0;
  if (v >= maxTick) return 1;

  const segments = ticks.length - 1;
  for (let i = 0; i < segments; i++) {
    const lo = ticks[i];
    const hi = ticks[i + 1];
    if (v <= hi) {
      const local = (v - lo) / (hi - lo);
      return (i + local) / segments;
    }
  }
  return 1;
}

export function fractionToAngleDeg(fraction: number): number {
  const clamped = Math.min(1, Math.max(0, fraction));
  return ARC_START_DEG + clamped * ARC_SWEEP_DEG;
}

export function valueToAngleDeg(value: number, scaleMax: number): number {
  return fractionToAngleDeg(valueToFraction(value, scaleMax));
}

/** Polar to screen-space Cartesian point (`0°` = right, clockwise). */
export function polarToCartesian(radius: number, angleDeg: number): { x: number; y: number } {
  const rad = (angleDeg * Math.PI) / 180;
  return { x: radius * Math.cos(rad), y: radius * Math.sin(rad) };
}
