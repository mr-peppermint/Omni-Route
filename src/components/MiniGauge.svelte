<script lang="ts">
  import { lang, t } from "../lib/i18n.svelte";
  import { untrack, type Snippet } from "svelte";
  import { formatMbps, speedParts } from "../lib/format";
  import { polarToCartesian, valueToFraction } from "../lib/scale";

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
    /** Variante compatta orizzontale (quadrante piccolo + testo a destra),
     * usata per i singoli link che confluiscono nel totale. */
    row?: boolean;
    /** Riga secondaria sotto il nome (solo variante `row`). */
    detail?: string;
    /** Override per distinguere il throughput testato dalla media live. */
    averageLabel?: string;
    children?: Snippet;
  } = $props();

  const SIZE = 220;
  const CENTER = SIZE / 2;
  const RADIUS = 82;
  const START = 145;
  const END = 395;

  function arcPath(startDeg: number, endDeg: number) {
    const start = polarToCartesian(RADIUS, startDeg);
    const end = polarToCartesian(RADIUS, endDeg);
    const largeArc = endDeg - startDeg > 180 ? 1 : 0;
    return `M ${CENTER + start.x} ${CENTER + start.y} A ${RADIUS} ${RADIUS} 0 ${largeArc} 1 ${CENTER + end.x} ${CENTER + end.y}`;
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
      const progress = Math.min(1, (now - startedAt) / 320);
      const eased = 1 - Math.pow(1 - progress, 3);
      displayed = startValue + (target - startValue) * eased;
      if (progress < 1) animationFrame = requestAnimationFrame(animate);
    }
    animationFrame = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animationFrame);
  });

  const progressAngle = $derived(
    START + (END - START) * valueToFraction(displayed, Math.max(10, scaleMax)),
  );
  const marker = $derived(polarToCartesian(RADIUS, progressAngle));
</script>

<article class="gauge" class:featured class:row class:offline={!online} style={`--link-color: ${color}`}>
  <svg viewBox={`0 0 ${SIZE} ${SIZE}`} role="img" aria-label={`${label}: ${formatMbps(value)}`}>
    <path class="track" d={arcPath(START, END)} />
    {#if progressAngle > START + 0.5}
      <!-- un arco di lunghezza zero con estremità arrotondate disegnerebbe un puntino -->
      <path class="value" d={arcPath(START, progressAngle)} />
    {/if}
    {#if online}
      <circle class="marker" cx={CENTER + marker.x} cy={CENTER + marker.y} r="4.5" />
    {/if}
    {#if !row}
      <text x={CENTER} y={CENTER - 2} text-anchor="middle" class="number">
        {online ? speedParts(displayed).value : "—"}
      </text>
      <text x={CENTER} y={CENTER + 20} text-anchor="middle" class="unit">
        {online ? speedParts(displayed).unit : t("Offline")}
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
    padding: 0.55rem 0.6rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: color-mix(in srgb, var(--surface-2) 85%, transparent);
    transition: border-color 200ms ease;
  }
  .gauge.featured {
    border-color: color-mix(in srgb, var(--accent) 22%, var(--border));
    background:
      radial-gradient(ellipse 80% 80% at 50% 100%,
        color-mix(in srgb, var(--accent) 5%, transparent), transparent),
      color-mix(in srgb, var(--accent) 4%, var(--surface-2));
  }
  .gauge.offline { opacity: 0.52; }
  svg { width: 100%; height: auto; display: block; margin: -0.22rem auto -1.1rem; }
  path { fill: none; stroke-width: 12; stroke-linecap: round; }
  .track { stroke: var(--gauge-track); }
  .value {
    stroke: var(--link-color);
    filter: drop-shadow(0 0 4px color-mix(in srgb, var(--link-color) 28%, transparent));
    transition: d 80ms ease;
  }
  .marker { fill: var(--text); }
  .number {
    fill: var(--text); font-family: var(--font-numeric);
    font-size: 30px; font-weight: 580; font-variant-numeric: tabular-nums lining-nums;
  }
  .unit { fill: var(--text-subtle); font-size: 10px; }
  .caption { min-width: 0; text-align: center; }
  .caption strong, .caption span { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .caption strong { color: var(--text); font-size: 0.74rem; }
  .caption span { margin-top: 0.14rem; color: var(--text-subtle); font-size: 0.62rem; }

  /* Row variant: compact horizontal */
  .gauge.row {
    height: 100%;
    display: grid; grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center; gap: 0.65rem;
    padding: 0.3rem 0.7rem 0.3rem 0.38rem;
    border-left: 2.5px solid var(--link-color);
  }
  .gauge.row svg { width: auto; height: 100%; max-height: 70px; min-height: 38px; aspect-ratio: 1; margin: 0; }
  .gauge.row path { stroke-width: 17; }
  .gauge.row .caption { text-align: left; }
  .gauge.row .caption strong { font-size: 0.77rem; }
  .gauge.row .now {
    display: block; margin-top: 0.08rem;
    color: var(--link-color); font-family: var(--font-numeric);
    font-size: 0.98rem; font-weight: 680; font-variant-numeric: tabular-nums lining-nums;
  }
  .gauge.row .caption span { margin-top: 0.07rem; }

  /* Featured (hub total gauge) */
  .gauge.featured:not(.row) { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; }
</style>
