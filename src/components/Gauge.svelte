<script lang="ts">
  import { lang, t } from "../lib/i18n.svelte";
  // Gauge radiale (§6.3 della spec). SVG scritto a mano, nessuna libreria.
  // Arco 270° (135°→405°), tre strati: traccia di fondo, archi dei link
  // impilati (cumulativi sulla scala log condivisa, cosi' la punta
  // dell'arco impilato coincide sempre col marcatore del totale), e il
  // marcatore stesso.

  import { untrack } from "svelte";
  import type { LinkStats } from "../lib/ipc";
  import { valueToFraction, fractionToAngleDeg, tickValues, polarToCartesian, ARC_START_DEG, SCALE_STEPS } from "../lib/scale";
  import { formatMbps, formatGainFactor, speedParts } from "../lib/format";

  let { links, totalMbps, totalAvgMbps, scaleMax, gainFactor }: {
    links: LinkStats[];
    totalMbps: number;
    totalAvgMbps: number;
    scaleMax: number;
    gainFactor: number;
  } = $props();

  const SIZE = 320;
  const CENTER = SIZE / 2;
  const RADIUS = 130;
  const TRACK_WIDTH = 20;

  function arcPath(radius: number, startDeg: number, endDeg: number): string {
    if (endDeg <= startDeg) return "";
    const start = polarToCartesian(radius, startDeg);
    const end = polarToCartesian(radius, endDeg);
    const largeArc = endDeg - startDeg > 180 ? 1 : 0;
    return `M ${CENTER + start.x} ${CENTER + start.y} A ${radius} ${radius} 0 ${largeArc} 1 ${CENTER + end.x} ${CENTER + end.y}`;
  }

  const fullTrackPath = $derived(arcPath(RADIUS, 135, 405));
  const testedTotal = $derived(
    links
      .filter((link) => link.state !== "disabled" && link.state !== "offline")
      .reduce((sum, link) => sum + (link.speedTestMbps ?? 0), 0),
  );
  const showingTest = $derived(totalMbps < 0.05 && testedTotal > 0);
  const displayTotal = $derived(showingTest ? testedTotal : totalMbps);
  const displayAverage = $derived(showingTest ? testedTotal : totalAvgMbps);
  const testScale = $derived(SCALE_STEPS.find((step) => step >= displayTotal * 1.15) ?? SCALE_STEPS[SCALE_STEPS.length - 1]);
  const displayScale = $derived(showingTest ? Math.max(scaleMax, testScale) : scaleMax);

  // Animazione: i campioni arrivano a 4Hz, interpoliamo con
  // requestAnimationFrame + easing (~180ms) per un movimento fluido.
  // prefers-reduced-motion: aggiornamento a scatti, senza interpolazione.
  const prefersReducedMotion =
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  let displayedTotal = $state(0);
  let targetTotal = $state(0);
  let animFrame = 0;

  $effect(() => {
    targetTotal = displayTotal;
  });

  $effect(() => {
    if (prefersReducedMotion) {
      displayedTotal = targetTotal;
      return;
    }
    // Smooth the actual gauge value, not just the pointer. Rises follow
    // quickly; drops settle more slowly so normal 4 Hz sampling jitter does
    // not make the fill visibly chatter. The current value is preserved when
    // a new sample arrives, so an update never restarts from the old target.
    let lastTime = performance.now();
    cancelAnimationFrame(animFrame);
    function step(now: number) {
      const elapsed = Math.min(0.1, (now - lastTime) / 1000);
      lastTime = now;
      const target = untrack(() => targetTotal);
      const current = untrack(() => displayedTotal);
      const settlingTime = target > current ? 0.14 : 0.42;
      const fraction = 1 - Math.exp(-elapsed / settlingTime);
      const next = current + (target - current) * fraction;
      displayedTotal = Math.abs(target - next) < 0.02 ? target : next;
      if (Math.abs(target - next) >= 0.02) animFrame = requestAnimationFrame(step);
    }
    animFrame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(animFrame);
  });

  // Archi impilati: ogni link parte da dove finisce la somma cumulativa
  // precedente (§6.3) — la punta dell'ultimo coincide col totale.
  const stackedArcs = $derived.by(() => {
    let cumulative = 0;
    const arcs: { key: string; color: string; d: string }[] = [];
    for (const l of links) {
      if (l.state === "disabled" || l.state === "offline") continue;
      const startFrac = valueToFraction(cumulative, displayScale);
      const rawTotal = Math.max(0, displayTotal);
      const linkSpeed = Math.max(0, showingTest ? (l.speedTestMbps ?? 0) : l.mbps);
      // Keep the stacked link colors aligned with the smoothed total endpoint.
      cumulative += rawTotal > 0 ? (linkSpeed / rawTotal) * displayedTotal : 0;
      const endFrac = valueToFraction(cumulative, displayScale);
      const startDeg = fractionToAngleDeg(startFrac);
      const endDeg = fractionToAngleDeg(endFrac);
      arcs.push({ key: l.key, color: l.color, d: arcPath(RADIUS, startDeg, endDeg) });
    }
    return arcs;
  });

  const needleAngle = $derived(fractionToAngleDeg(valueToFraction(displayedTotal, displayScale)));
  const needleTip = $derived(polarToCartesian(RADIUS, needleAngle));

  const ticks = $derived(
    tickValues(displayScale).map((v) => {
      const angle = fractionToAngleDeg(valueToFraction(v, displayScale));
      const inner = polarToCartesian(RADIUS - TRACK_WIDTH / 2 - 4, angle);
      const outer = polarToCartesian(RADIUS + TRACK_WIDTH / 2 + 4, angle);
      const label = polarToCartesian(RADIUS + TRACK_WIDTH / 2 + 18, angle);
      return { v, inner, outer, label };
    }),
  );
  const majorTickFractions = $derived(
    ticks.map((tick) => valueToFraction(tick.v, displayScale)),
  );
  const minorTicks = $derived.by(() => {
    const marks: { inner: { x: number; y: number }; outer: { x: number; y: number } }[] = [];
    for (let i = 1; i < 20; i += 1) {
      const fraction = i / 20;
      if (majorTickFractions.some((major) => Math.abs(major - fraction) < 0.014)) continue;
      const angle = fractionToAngleDeg(fraction);
      marks.push({
        inner: polarToCartesian(RADIUS - TRACK_WIDTH / 2 - 2, angle),
        outer: polarToCartesian(RADIUS + TRACK_WIDTH / 2 + 2, angle),
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
</script>

<svg
  style="overflow: visible"
  width={SIZE}
  height={SIZE}
  viewBox="0 0 {SIZE} {SIZE}"
  role="img"
  aria-label={hasActiveLinks
    ? showingTest
      ? t("Velocità testata {total}", { total: formatMbps(displayTotal) })
      : t("Velocità totale {total}, media {avg}, {gain}", { total: formatMbps(displayTotal), avg: formatMbps(displayAverage), gain: formatGainFactor(gainFactor) })
    : t("Nessuna connessione attiva")}
  class:paused={!hasActiveLinks}
>
  <path
    d={fullTrackPath}
    fill="none"
    stroke="var(--gauge-track)"
    stroke-width={TRACK_WIDTH}
    stroke-linecap="round"
  />

  {#each minorTicks as tick}
    <line
      x1={CENTER + tick.inner.x}
      y1={CENTER + tick.inner.y}
      x2={CENTER + tick.outer.x}
      y2={CENTER + tick.outer.y}
      stroke="var(--gauge-tick)"
      stroke-width="1"
      stroke-linecap="round"
      class="minor-tick"
    />
  {/each}

  {#each stackedArcs as arc (arc.key)}
    <path d={arc.d} fill="none" stroke={arc.color} stroke-width={TRACK_WIDTH} stroke-linecap="butt" />
  {/each}

  {#each ticks as tick (tick.v)}
    <line
      x1={CENTER + tick.inner.x}
      y1={CENTER + tick.inner.y}
      x2={CENTER + tick.outer.x}
      y2={CENTER + tick.outer.y}
      stroke="var(--gauge-tick)"
      stroke-width="1.5"
      stroke-linecap="round"
    />
    <text
      x={CENTER + tick.label.x}
      y={CENTER + tick.label.y}
      fill="var(--text-muted)"
      font-size="11"
      text-anchor="middle"
      dominant-baseline="middle"
    >
      {tick.v}
    </text>
  {/each}

  {#if hasActiveLinks}
    <circle
      cx={CENTER + needleTip.x}
      cy={CENTER + needleTip.y}
      r="6"
      fill="var(--text)"
      class="needle-glow"
    />
  {/if}

  <text x={CENTER} y={CENTER - 8} text-anchor="middle" class="center-value">
    {hasActiveLinks ? speedParts(displayedTotal).value : "—"}
  </text>
  <text x={CENTER} y={CENTER + 16} text-anchor="middle" class="center-unit">
    {hasActiveLinks ? speedParts(displayedTotal).unit : t("Nessuna connessione")}
  </text>
  {#if hasActiveLinks}
    <text x={CENTER} y={CENTER + 38} text-anchor="middle" class="center-gain">
      {showingTest
        ? t("Speed test · max 10 MB per link")
        : t("{name} · media {avg}", { name: singleLinkName ?? formatGainFactor(gainFactor), avg: avgLabel(displayAverage) })}
    </text>
  {/if}
</svg>

<style>
  svg { display: block; }
  svg.paused { opacity: 0.52; }

  .center-value {
    font-family: var(--font-numeric);
    font-size: 42px; font-weight: 540;
    font-variant-numeric: tabular-nums;
    fill: var(--text);
  }
  .center-unit { font-size: 13px; fill: var(--text-muted); }
  .center-gain { font-size: 11px; fill: var(--text-subtle); }
  .minor-tick { opacity: 0.62; }

  .needle-glow {
    filter: drop-shadow(0 0 5px color-mix(in srgb, var(--accent) 55%, transparent));
    animation: needle-pulse 2s ease-in-out infinite;
  }
  @keyframes needle-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.75; }
  }
  @media (prefers-reduced-motion: reduce) {
    .needle-glow { filter: none; animation: none; }
  }
</style>
