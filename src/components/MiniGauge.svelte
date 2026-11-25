<script lang="ts">
  import { untrack, type Snippet } from "svelte";
  import { t } from "../lib/i18n.svelte";
  import { formatMbps, speedParts } from "../lib/format";
  import {
    ARC_END_DEG,
    ARC_START_DEG,
    formatTickLabel,
    fractionToAngleDeg,
    polarToCartesian,
    tickValues,
    valueToFraction,
  } from "../lib/scale";

  let {
    label,
    value,
    average,
    scaleMax,
    color = "var(--accent)",
    online = true,
    featured = false,
    row = false,
    detail,
    averageLabel,
    children,
  }: {
    label: string;
    value: number;
    average: number;
    scaleMax: number;
    color?: string;
    online?: boolean;
    featured?: boolean;
    row?: boolean;
    detail?: string;
    averageLabel?: string;
    children?: Snippet;
  } = $props();

  // Unique SVG gradient ID per component instance
  const uid = Math.random().toString(36).slice(2, 8);
  const gradId = `mini-grad-${uid}`;

  // Geometry differs cleanly between featured semicircle speedometer and compact row arc
  const WIDTH = $derived(row ? 88 : 320);
  const HEIGHT = $derived(row ? 54 : 204);
  const CX = $derived(WIDTH / 2);
  const CY = $derived(row ? 46 : 168);
  const RADIUS = $derived(row ? 34 : 126);
  const TRACK_WIDTH = $derived(row ? 8 : 17);

  function arcPath(cx: number, cy: number, radius: number, startDeg: number, endDeg: number): string {
    if (endDeg <= startDeg + 0.05) return "";
    const start = polarToCartesian(radius, startDeg);
    const end = polarToCartesian(radius, endDeg);
    const largeArc = endDeg - startDeg > 180 ? 1 : 0;
    return `M ${(cx + start.x).toFixed(2)} ${(cy + start.y).toFixed(2)} A ${radius} ${radius} 0 ${largeArc} 1 ${(cx + end.x).toFixed(2)} ${(cy + end.y).toFixed(2)}`;
  }

  let displayed = $state(0);
  let animationFrame = 0;
  const reducedMotion =
    typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  $effect(() => {
    const target = Math.max(0, value);
    if (reducedMotion) {
      displayed = target;
      return;
    }
    const startValue = untrack(() => displayed);
    const startedAt = performance.now();
    cancelAnimationFrame(animationFrame);
    function animate(now: number) {
      const progress = Math.min(1, (now - startedAt) / 300);
      const eased = 1 - Math.pow(1 - progress, 3);
      displayed = startValue + (target - startValue) * eased;
      if (progress < 1) animationFrame = requestAnimationFrame(animate);
    }
    animationFrame = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animationFrame);
  });

  const effectiveScale = $derived(Math.max(1000, scaleMax));
  const frac = $derived(valueToFraction(displayed, effectiveScale));
  const progressAngle = $derived(fractionToAngleDeg(frac));

  const fullTrack = $derived(arcPath(CX, CY, RADIUS, ARC_START_DEG, ARC_END_DEG));
  const innerRim = $derived(arcPath(CX, CY, RADIUS - TRACK_WIDTH / 2, ARC_START_DEG, ARC_END_DEG));
  const progressArc = $derived(arcPath(CX, CY, RADIUS, ARC_START_DEG, progressAngle));

  const pointerInner = $derived(polarToCartesian(RADIUS - TRACK_WIDTH / 2 - 3.5, progressAngle));
  const pointerOuter = $derived(polarToCartesian(RADIUS + TRACK_WIDTH / 2 + 3.5, progressAngle));
  const pointerMid = $derived(polarToCartesian(RADIUS, progressAngle));

  const avgAngle = $derived(fractionToAngleDeg(valueToFraction(Math.max(0, average), effectiveScale)));
  const avgInner = $derived(polarToCartesian(RADIUS + TRACK_WIDTH / 2 + 1.5, avgAngle));
  const avgOuter = $derived(polarToCartesian(RADIUS + TRACK_WIDTH / 2 + 6.5, avgAngle));

  const majorTicks = $derived.by(() => {
    const values = tickValues(effectiveScale);
    const lastIdx = values.length - 1;
    return values.map((v, i) => {
      const angle = fractionToAngleDeg(i / lastIdx);
      const inner = polarToCartesian(RADIUS - TRACK_WIDTH / 2 - (row ? 3 : 5.5), angle);
      const outer = polarToCartesian(RADIUS - TRACK_WIDTH / 2, angle);
      const labelAngle = i === 0 ? ARC_START_DEG + 4.5 : i === lastIdx ? ARC_END_DEG - 4.5 : angle;
      const labelPos = polarToCartesian(RADIUS - TRACK_WIDTH / 2 - 19, labelAngle);
      const reached = displayed >= Math.max(0.05, v) && (v === 0 ? displayed > 0.05 : true);
      return { v, label: formatTickLabel(v), inner, outer, labelPos, reached };
    });
  });

  const minorTicks = $derived.by(() => {
    if (row) return [];
    const values = tickValues(effectiveScale);
    const segments = values.length - 1;
    const subdivisions = 4;
    const totalSub = segments * subdivisions;
    const marks: { inner: { x: number; y: number }; outer: { x: number; y: number } }[] = [];
    for (let i = 1; i < totalSub; i++) {
      if (i % subdivisions === 0) continue;
      const angle = fractionToAngleDeg(i / totalSub);
      marks.push({
        inner: polarToCartesian(RADIUS - TRACK_WIDTH / 2 - 3, angle),
        outer: polarToCartesian(RADIUS - TRACK_WIDTH / 2, angle),
      });
    }
    return marks;
  });

  const parts = $derived(speedParts(displayed));
</script>

<article class="gauge" class:featured class:row class:offline={!online} style={`--link-color: ${color}`}>
  <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} role="img" aria-label={`${label}: ${formatMbps(value)}`}>
    <defs>
      <linearGradient
        id={gradId}
        gradientUnits="userSpaceOnUse"
        x1={CX - RADIUS}
        y1={CY}
        x2={CX + RADIUS}
        y2={CY}
      >
        <stop offset="0%" stop-color={featured ? "var(--accent)" : color} />
        <stop offset="100%" stop-color={featured ? "var(--accent-2)" : color} />
      </linearGradient>
    </defs>

    <!-- Base semicircular track -->
    <path
      class="track"
      d={fullTrack}
      stroke-width={TRACK_WIDTH}
      stroke-linecap="butt"
    />

    <!-- Filled speed arc -->
    {#if progressArc}
      <path
        class="value"
        d={progressArc}
        stroke={`url(#${gradId})`}
        stroke-width={TRACK_WIDTH}
        stroke-linecap="butt"
      />
    {/if}

    {#if !row}
      <!-- Subtle inner rim guide -->
      <path
        d={innerRim}
        fill="none"
        stroke="var(--border-strong)"
        stroke-width="1"
        opacity="0.45"
      />

      <!-- Minor tick subdivisions -->
      {#each minorTicks as tick}
        <line
          x1={CX + tick.inner.x}
          y1={CY + tick.inner.y}
          x2={CX + tick.outer.x}
          y2={CY + tick.outer.y}
          stroke="var(--gauge-tick)"
          stroke-width="1"
          stroke-linecap="round"
          opacity="0.6"
        />
      {/each}
    {/if}

    <!-- Landmark ticks (+ readable numbers when not compact row) -->
    {#each majorTicks as tick (tick.v)}
      <line
        x1={CX + tick.inner.x}
        y1={CY + tick.inner.y}
        x2={CX + tick.outer.x}
        y2={CY + tick.outer.y}
        stroke={tick.reached && online ? "var(--text-muted)" : "var(--gauge-tick)"}
        stroke-width={row ? 1 : 1.5}
        stroke-linecap="round"
        opacity={row ? 0.7 : 1}
      />
      {#if !row}
        <text
          x={CX + tick.labelPos.x}
          y={CY + tick.labelPos.y}
          text-anchor="middle"
          dominant-baseline="middle"
          class="tick-label"
          class:reached={tick.reached && online}
        >
          {tick.label}
        </text>
      {/if}
    {/each}

    <!-- Average outer marker on full view -->
    {#if !row && online && average > 0.25}
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

    <!-- Active speed indicator -->
    {#if online}
      {#if row}
        <circle class="marker-row" cx={CX + pointerMid.x} cy={CY + pointerMid.y} r="3.6" />
      {:else}
        <line
          x1={CX + pointerInner.x}
          y1={CY + pointerInner.y}
          x2={CX + pointerOuter.x}
          y2={CY + pointerOuter.y}
          stroke="var(--text)"
          stroke-width="2.6"
          stroke-linecap="round"
        />
        <circle
          class="marker"
          cx={CX + pointerMid.x}
          cy={CY + pointerMid.y}
          r="4.5"
        />
      {/if}
    {/if}

    <!-- Center numeric readout for featured / standalone semicircle -->
    {#if !row}
      <text x={CX} y={CY - 28} text-anchor="middle" class="number">
        {online ? parts.value : "—"}
      </text>
      <text x={CX} y={CY - 4} text-anchor="middle" class="unit">
        {online ? parts.unit : t("Offline")}
      </text>
    {/if}
  </svg>

  <div class="caption">
    <strong>{label}</strong>
    {#if row}
      <b class="now">{online ? formatMbps(displayed) : "—"}</b>
      <span>{[detail, online ? (averageLabel ?? t("media {v}", { v: formatMbps(average) })) : ""].filter(Boolean).join(" · ")}</span>
    {:else}
      <span>{online ? t("Media {v}", { v: formatMbps(average) }) : featured ? t("Nessuna connessione attiva") : t("Connessione memorizzata")}</span>
    {/if}
  </div>
  {@render children?.()}
</article>

<style>
  .gauge {
    min-width: 0;
    padding: 0.65rem 0.75rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--surface-2) 85%, transparent);
    transition: border-color 200ms ease;
  }
  .gauge.featured {
    border-color: color-mix(in srgb, var(--accent) 22%, var(--border));
    background:
      radial-gradient(ellipse 80% 70% at 50% 95%,
        color-mix(in srgb, var(--accent) 6%, transparent), transparent),
      color-mix(in srgb, var(--accent) 3%, var(--surface-2));
  }
  .gauge.offline { opacity: 0.52; }

  svg {
    width: 100%;
    height: auto;
    display: block;
    overflow: visible;
    margin: 0 auto;
  }
  path { fill: none; }
  .track { stroke: var(--gauge-track); }
  .value {
    filter: drop-shadow(0 0 4px color-mix(in srgb, var(--link-color) 26%, transparent));
  }

  .tick-label {
    font-family: var(--font-numeric);
    font-size: 9.8px;
    font-weight: 550;
    font-variant-numeric: tabular-nums;
    fill: var(--text-subtle);
    transition: fill 200ms ease;
  }
  .tick-label.reached {
    fill: var(--text-muted);
    font-weight: 650;
  }

  .marker {
    fill: var(--text);
    stroke: var(--surface-1);
    stroke-width: 1.8;
    filter: drop-shadow(0 0 4px color-mix(in srgb, var(--accent) 40%, transparent));
  }
  .marker-row {
    fill: var(--text);
    stroke: var(--surface-2);
    stroke-width: 1.4;
  }

  .number {
    fill: var(--text);
    font-family: var(--font-numeric);
    font-size: 40px;
    font-weight: 580;
    letter-spacing: -0.025em;
    font-variant-numeric: tabular-nums lining-nums;
  }
  .unit {
    fill: var(--text-muted);
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .caption { min-width: 0; text-align: center; margin-top: 0.15rem; }
  .caption strong, .caption span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .caption strong { color: var(--text); font-size: 0.76rem; }
  .caption span { margin-top: 0.14rem; color: var(--text-subtle); font-size: 0.64rem; }

  /* Row variant: compact horizontal with mini semicircle */
  .gauge.row {
    height: 100%;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.7rem;
    padding: 0.35rem 0.75rem 0.35rem 0.5rem;
    border-left: 2.5px solid var(--link-color);
  }
  .gauge.row svg {
    width: 68px;
    height: auto;
    aspect-ratio: 88 / 54;
    margin: 0;
  }
  .gauge.row .caption { text-align: left; margin-top: 0; }
  .gauge.row .caption strong { font-size: 0.77rem; }
  .gauge.row .now {
    display: block;
    margin-top: 0.08rem;
    color: var(--link-color);
    font-family: var(--font-numeric);
    font-size: 0.98rem;
    font-weight: 680;
    font-variant-numeric: tabular-nums lining-nums;
  }
  .gauge.row .caption span { margin-top: 0.07rem; }

  /* Featured (hub total semicircle gauge) */
  .gauge.featured:not(.row) {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.2rem;
  }

  @media (prefers-reduced-motion: reduce) {
    .value, .marker { filter: none; }
  }
</style>
