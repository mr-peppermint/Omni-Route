<script lang="ts">
  import { untrack } from "svelte";
  import { t } from "../lib/i18n.svelte";
  import type { LinkStats } from "../lib/ipc";
  import {
    ARC_START_DEG,
    ARC_END_DEG,
    formatTickLabel,
    fractionToAngleDeg,
    polarToCartesian,
    SCALE_STEPS,
    tickValues,
    valueToFraction,
  } from "../lib/scale";
  import { formatGainFactor, formatMbps, speedParts } from "../lib/format";

  let { links, totalMbps, totalAvgMbps, scaleMax, gainFactor }: {
    links: LinkStats[];
    totalMbps: number;
    totalAvgMbps: number;
    scaleMax: number;
    gainFactor: number;
  } = $props();

  // 180° semicircle gauge geometry
  const WIDTH = 360;
  const HEIGHT = 222;
  const CX = WIDTH / 2;
  const CY = 184;
  const RADIUS = 142;
  const TRACK_WIDTH = 20;

  function arcPath(radius: number, startDeg: number, endDeg: number): string {
    if (endDeg <= startDeg + 0.05) return "";
    const start = polarToCartesian(radius, startDeg);
    const end = polarToCartesian(radius, endDeg);
    const largeArc = endDeg - startDeg > 180 ? 1 : 0;
    return `M ${(CX + start.x).toFixed(2)} ${(CY + start.y).toFixed(2)} A ${radius} ${radius} 0 ${largeArc} 1 ${(CX + end.x).toFixed(2)} ${(CY + end.y).toFixed(2)}`;
  }

  const fullTrackPath = arcPath(RADIUS, ARC_START_DEG, ARC_END_DEG);
  const innerRimPath = arcPath(RADIUS - TRACK_WIDTH / 2, ARC_START_DEG, ARC_END_DEG);

  const testedTotal = $derived(
    links
      .filter((link) => link.state !== "disabled" && link.state !== "offline")
      .reduce((sum, link) => sum + (link.speedTestMbps ?? 0), 0),
  );
  const showingTest = $derived(totalMbps < 0.05 && testedTotal > 0);
  const displayTotal = $derived(showingTest ? testedTotal : totalMbps);
  const displayAverage = $derived(showingTest ? testedTotal : totalAvgMbps);
  const testScale = $derived(
    SCALE_STEPS.find((step) => step >= displayTotal * 1.15) ?? SCALE_STEPS[SCALE_STEPS.length - 1],
  );
  const displayScale = $derived(Math.max(1000, showingTest ? Math.max(scaleMax, testScale) : scaleMax));

  const prefersReducedMotion =
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let displayedTotal = $state(0);
  let targetTotal = $state(0);
  let animFrame = 0;

  $effect(() => {
    targetTotal = Math.max(0, displayTotal);
  });

  $effect(() => {
    if (prefersReducedMotion) {
      displayedTotal = targetTotal;
      return;
    }
    let lastTime = performance.now();
    cancelAnimationFrame(animFrame);
    function step(now: number) {
      const elapsed = Math.min(0.1, (now - lastTime) / 1000);
      lastTime = now;
      const target = untrack(() => targetTotal);
      const current = untrack(() => displayedTotal);
      const settlingTime = target > current ? 0.14 : 0.38;
      const fraction = 1 - Math.exp(-elapsed / settlingTime);
      const next = current + (target - current) * fraction;
      displayedTotal = Math.abs(target - next) < 0.02 ? target : next;
      if (Math.abs(target - next) >= 0.02) animFrame = requestAnimationFrame(step);
    }
    animFrame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(animFrame);
  });

  // Stacked colored link segments aligned to the smoothed displayedTotal
  const stackedArcs = $derived.by(() => {
    let cumulative = 0;
    const arcs: { key: string; color: string; d: string }[] = [];
    const rawTotal = Math.max(0, displayTotal);
    for (const l of links) {
      if (l.state === "disabled" || l.state === "offline") continue;
      const linkSpeed = Math.max(0, showingTest ? (l.speedTestMbps ?? 0) : l.mbps);
      const share = rawTotal > 0 ? (linkSpeed / rawTotal) * displayedTotal : 0;
      if (share <= 0.005) continue;
      const startFrac = valueToFraction(cumulative, displayScale);
      cumulative += share;
      const endFrac = valueToFraction(cumulative, displayScale);
      const d = arcPath(RADIUS, fractionToAngleDeg(startFrac), fractionToAngleDeg(endFrac));
      if (d) arcs.push({ key: l.key, color: l.color, d });
    }
    return arcs;
  });

  const activeFraction = $derived(valueToFraction(displayedTotal, displayScale));
  const pointerAngle = $derived(fractionToAngleDeg(activeFraction));
  const pointerInner = $derived(polarToCartesian(RADIUS - TRACK_WIDTH / 2 - 4, pointerAngle));
  const pointerOuter = $derived(polarToCartesian(RADIUS + TRACK_WIDTH / 2 + 4, pointerAngle));
  const pointerMid = $derived(polarToCartesian(RADIUS, pointerAngle));

  const avgAngle = $derived(fractionToAngleDeg(valueToFraction(displayAverage, displayScale)));
  const avgInner = $derived(polarToCartesian(RADIUS + TRACK_WIDTH / 2 + 2, avgAngle));
  const avgOuter = $derived(polarToCartesian(RADIUS + TRACK_WIDTH / 2 + 8, avgAngle));

  // Google-style inner network scale ticks & labels
  const majorTicks = $derived.by(() => {
    const values = tickValues(displayScale);
    const lastIdx = values.length - 1;
    return values.map((v, i) => {
      const frac = i / lastIdx;
      const angle = fractionToAngleDeg(frac);
      const inner = polarToCartesian(RADIUS - TRACK_WIDTH / 2 - 6, angle);
      const outer = polarToCartesian(RADIUS - TRACK_WIDTH / 2, angle);
      // Slight inward/upward nudge on first & last labels so 0 and 1,000 sit cleanly above baseline
      const labelAngle = i === 0 ? ARC_START_DEG + 4 : i === lastIdx ? ARC_END_DEG - 4 : angle;
      const labelPos = polarToCartesian(RADIUS - TRACK_WIDTH / 2 - 22, labelAngle);
      const reached = displayedTotal >= Math.max(0.05, v) && (v === 0 ? displayedTotal > 0.05 : true);
      return {
        v,
        label: formatTickLabel(v),
        inner,
        outer,
        labelPos,
        reached,
      };
    });
  });

  const minorTicks = $derived.by(() => {
    const values = tickValues(displayScale);
    const segments = values.length - 1;
    const subdivisions = 4;
    const totalSub = segments * subdivisions;
    const marks: { inner: { x: number; y: number }; outer: { x: number; y: number } }[] = [];
    for (let i = 1; i < totalSub; i++) {
      if (i % subdivisions === 0) continue;
      const angle = fractionToAngleDeg(i / totalSub);
      marks.push({
        inner: polarToCartesian(RADIUS - TRACK_WIDTH / 2 - 3.5, angle),
        outer: polarToCartesian(RADIUS - TRACK_WIDTH / 2, angle),
      });
    }
    return marks;
  });

  function avgLabel(mbps: number) {
    const p = speedParts(mbps);
    return p.unit === "Gbps" ? `${p.value} Gbps` : p.value;
  }

  const activeLinks = $derived(links.filter((l) => l.state !== "disabled" && l.state !== "offline"));
  const hasActiveLinks = $derived(activeLinks.length > 0);
  const singleLinkName = $derived(activeLinks.length === 1 ? activeLinks[0].name : null);
  const parts = $derived(speedParts(displayedTotal));
</script>

<svg
  viewBox="0 0 {WIDTH} {HEIGHT}"
  role="img"
  aria-label={hasActiveLinks
    ? showingTest
      ? t("Velocità testata {total}", { total: formatMbps(displayTotal) })
      : t("Velocità totale {total}, media {avg}, {gain}", {
          total: formatMbps(displayTotal),
          avg: formatMbps(displayAverage),
          gain: formatGainFactor(gainFactor),
        })
    : t("Nessuna connessione attiva")}
  class:paused={!hasActiveLinks}
>
  <!-- Base semicircle track -->
  <path
    d={fullTrackPath}
    fill="none"
    stroke="var(--gauge-track)"
    stroke-width={TRACK_WIDTH}
    stroke-linecap="butt"
  />

  <!-- Stacked active link arcs -->
  {#each stackedArcs as arc (arc.key)}
    <path
      d={arc.d}
      fill="none"
      stroke={arc.color}
      stroke-width={TRACK_WIDTH}
      stroke-linecap="butt"
      class="active-arc"
    />
  {/each}

  <!-- Subtle inner rim guide -->
  <path
    d={innerRimPath}
    fill="none"
    stroke="var(--border-strong)"
    stroke-width="1"
    opacity="0.5"
  />

  <!-- Minor subdivision ticks along inner rim -->
  {#each minorTicks as tick}
    <line
      x1={CX + tick.inner.x}
      y1={CY + tick.inner.y}
      x2={CX + tick.outer.x}
      y2={CY + tick.outer.y}
      stroke="var(--gauge-tick)"
      stroke-width="1"
      stroke-linecap="round"
      opacity="0.65"
    />
  {/each}

  <!-- Major network landmark ticks and inner labels -->
  {#each majorTicks as tick (tick.v)}
    <line
      x1={CX + tick.inner.x}
      y1={CY + tick.inner.y}
      x2={CX + tick.outer.x}
      y2={CY + tick.outer.y}
      stroke={tick.reached && hasActiveLinks ? "var(--text-muted)" : "var(--gauge-tick)"}
      stroke-width="1.6"
      stroke-linecap="round"
    />
    <text
      x={CX + tick.labelPos.x}
      y={CY + tick.labelPos.y}
      text-anchor="middle"
      dominant-baseline="middle"
      class="tick-label"
      class:reached={tick.reached && hasActiveLinks}
    >
      {tick.label}
    </text>
  {/each}

  <!-- 10s average outer marker -->
  {#if hasActiveLinks && displayAverage > 0.25}
    <line
      x1={CX + avgInner.x}
      y1={CY + avgInner.y}
      x2={CX + avgOuter.x}
      y2={CY + avgOuter.y}
      stroke="var(--accent-2)"
      stroke-width="2"
      stroke-linecap="round"
      opacity="0.85"
    />
  {/if}

  <!-- Live rim pointer -->
  {#if hasActiveLinks}
    <g class="pointer">
      <line
        x1={CX + pointerInner.x}
        y1={CY + pointerInner.y}
        x2={CX + pointerOuter.x}
        y2={CY + pointerOuter.y}
        stroke="var(--text)"
        stroke-width="3"
        stroke-linecap="round"
      />
      <circle
        cx={CX + pointerMid.x}
        cy={CY + pointerMid.y}
        r="5"
        fill="var(--text)"
        stroke="var(--surface-1)"
        stroke-width="2"
        class="pointer-dot"
      />
    </g>
  {/if}

  <!-- Central digital speed readout -->
  <text x={CX} y={CY - 32} text-anchor="middle" class="center-value">
    {hasActiveLinks ? parts.value : "—"}
  </text>
  <text x={CX} y={CY - 7} text-anchor="middle" class="center-unit">
    {hasActiveLinks ? parts.unit : t("Nessuna connessione")}
  </text>
  {#if hasActiveLinks}
    <text x={CX} y={CY + 18} text-anchor="middle" class="center-meta">
      {showingTest
        ? t("Speed test · max 10 MB per link")
        : t("{name} · media {avg}", {
            name: singleLinkName ?? formatGainFactor(gainFactor),
            avg: avgLabel(displayAverage),
          })}
    </text>
  {/if}
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
  }
  svg.paused {
    opacity: 0.54;
  }

  .active-arc {
    transition: opacity 200ms ease;
  }

  .tick-label {
    font-family: var(--font-numeric);
    font-size: 11px;
    font-weight: 550;
    font-variant-numeric: tabular-nums;
    fill: var(--text-subtle);
    transition: fill 220ms ease;
  }
  .tick-label.reached {
    fill: var(--text-muted);
    font-weight: 650;
  }

  .center-value {
    font-family: var(--font-numeric);
    font-size: 46px;
    font-weight: 580;
    letter-spacing: -0.025em;
    font-variant-numeric: tabular-nums lining-nums;
    fill: var(--text);
  }
  .center-unit {
    font-size: 13px;
    font-weight: 600;
    letter-spacing: 0.02em;
    fill: var(--text-muted);
  }
  .center-meta {
    font-size: 11px;
    fill: var(--text-subtle);
  }

  .pointer-dot {
    filter: drop-shadow(0 0 4px color-mix(in srgb, var(--accent) 45%, transparent));
    animation: pointer-pulse 2.2s ease-in-out infinite;
  }
  @keyframes pointer-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.78; }
  }
  @media (prefers-reduced-motion: reduce) {
    .pointer-dot {
      filter: none;
      animation: none;
    }
  }
</style>
