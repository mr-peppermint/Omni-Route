<script lang="ts">
  import { lang, t } from "../lib/i18n.svelte";
  import { formatMbps } from "../lib/format";
  import { systemBalance, monitor } from "../lib/stores.svelte";
  import { SCALE_STEPS } from "../lib/scale";
  import Icon from "./Icon.svelte";
  import MiniGauge from "./MiniGauge.svelte";

  const status = $derived(systemBalance.status);
  const active = $derived(status?.state === "active");
  const availableLinks = $derived(
    (status?.links ?? [])
      .filter((link) => link.enabled && link.state === "online")
      .sort((left, right) => left.name.localeCompare(right.name, "en")),
  );
  // link presenti ma disattivati dall'utente: restano in elenco, attenuati e
  // con l'interruttore spento, così si possono riaccendere da qui
  const disabledLinks = $derived((status?.links ?? []).filter((link) => !link.enabled && link.state !== "offline"));
  const feedLinks = $derived(
    [...availableLinks, ...disabledLinks].sort((left, right) => left.name.localeCompare(right.name, "en")),
  );
  // Velocità mostrate: traffico reale di ogni scheda, misurato dal monitor
  // sempre, dall'avvio, anche a tunnel fermo o con il link escluso. Il
  // servizio conta solo il traffico che passa da Omni-Route: resta il ripiego
  // se il monitor non ha (ancora) un campione per quel link.
  function liveOf(link: { key: string; mbps: number; avgMbps: number }) {
    const measured = monitor.snapshot?.links.find((entry) => entry.key === link.key);
    return measured && measured.state !== "offline" && typeof measured.liveMbps === "number"
      ? {
          mbps: measured.liveMbps,
          avg: measured.liveAvgMbps,
          test: measured.speedTestMbps ?? null,
          testPending: measured.speedTestPending,
          testAttempted: measured.speedTestAttempted,
        }
      : { mbps: link.mbps, avg: link.avgMbps, test: null, testPending: false, testAttempted: false };
  }
  const live = $derived(new Map(feedLinks.map((link) => [link.key, liveOf(link)])));
  const totalNow = $derived([...live.values()].reduce((sum, rate) => sum + rate.mbps, 0));
  const totalAvg = $derived([...live.values()].reduce((sum, rate) => sum + rate.avg, 0));
  const scaleMax = $derived.by(() => {
    const peak = Math.max(totalNow, ...[...live.values()].map((rate) => Math.max(rate.mbps, rate.test ?? 0)), 0);
    return SCALE_STEPS.find((step) => step >= peak * 1.15) ?? SCALE_STEPS[SCALE_STEPS.length - 1];
  });
  const palette = ["#8b7cf8", "#d4954a", "#74b98a", "#c47ab5", "#6aaed4", "#d47a7a"];

  function linkColor(key: string) {
    const index = Math.max(0, feedLinks.findIndex((link) => link.key === key));
    return monitor.snapshot?.links.find((link) => link.key === key)?.color ?? palette[index % palette.length];
  }

  function meterValue(key: string) {
    const rate = live.get(key);
    return rate ? (rate.mbps >= 0.05 ? rate.mbps : rate.test ?? 0) : 0;
  }

  function meterAverageLabel(key: string) {
    const rate = live.get(key);
    if (!rate || rate.mbps >= 0.05) return undefined;
    if (rate.test != null) return t("Ultimo speed test: {v}", { v: formatMbps(rate.test) });
    if (rate.testPending) return t("Test velocità in corso…");
    if (rate.testAttempted) return t("Test velocità non disponibile");
    return undefined;
  }

  function stateLabel() {
    if (!status?.serviceAvailable) return t("Servizio non disponibile");
    if (status.state === "active") return t("Tutto il PC è bilanciato");
    if (status.state === "starting") return t("Attivazione in corso…");
    if (status.state === "waiting") return t("In attesa di una connessione utilizzabile");
    if (status.state === "error") return t("Intervento richiesto");
    return t("Pronto");
  }

  function errorTitle() {
    return status?.state === "error" ? t("Il tunnel non è partito.") : t("Il servizio non risponde.");
  }

  function kindLabel(kind: string) {
    if (kind === "usb") return "USB";
    if (kind === "wifi") return "Wi-Fi";
    if (kind === "ethernet") return "Ethernet";
    return t("Rete");
  }

  // ---- linee che confluiscono nel tachimetro totale ----------------------
  // Ogni link ha la sua linea: parte dal bordo destro della sua scheda ed
  // entra nel tachimetro grande in un punto tutto suo, con un tracciato
  // squadrato _|‾. Il tratteggio si muove verso il totale, più veloce quando
  // il link trasporta più traffico, fermo quando è a zero.
  type Flow = { key: string; d: string; color: string; duration: number; idle: boolean; end: { x: number; y: number } };

  let stageEl = $state<HTMLDivElement | null>(null);
  let hubEl = $state<HTMLDivElement | null>(null);
  let geometry = $state<{
    width: number;
    height: number;
    points: Map<string, { x: number; y: number }>;
    hub: { left: number; top: number; height: number };
  } | null>(null);

  function measure() {
    if (!stageEl || !hubEl) return;
    const stage = stageEl.getBoundingClientRect();
    const hub = (hubEl.querySelector(".gauge") ?? hubEl).getBoundingClientRect();
    const points = new Map<string, { x: number; y: number }>();
    stageEl.querySelectorAll<HTMLElement>("[data-feed]").forEach((feed) => {
      const rect = feed.getBoundingClientRect();
      points.set(feed.dataset.feed!, { x: rect.right - stage.left, y: rect.top + rect.height / 2 - stage.top });
    });
    geometry = {
      width: stage.width,
      height: stage.height,
      points,
      hub: { left: hub.left - stage.left, top: hub.top - stage.top, height: hub.height },
    };
  }

  $effect(() => {
    if (!stageEl) return;
    const observer = new ResizeObserver(() => measure());
    observer.observe(stageEl);
    return () => observer.disconnect();
  });

  // ri-misura quando cambia l'insieme dei link visibili
  $effect(() => {
    feedLinks.map((link) => `${link.key}:${link.enabled}`).join("|");
    queueMicrotask(measure);
  });

  function squarePath(x1: number, y1: number, x2: number, y2: number, mx: number) {
    const dy = y2 - y1;
    if (Math.abs(dy) < 1) return `M ${x1} ${y1} H ${x2}`;
    const dir = Math.sign(dy);
    const r = Math.max(0, Math.min(8, Math.abs(dy) / 2, mx - x1, x2 - mx));
    return `M ${x1} ${y1} H ${mx - r} Q ${mx} ${y1} ${mx} ${y1 + dir * r} V ${y2 - dir * r} Q ${mx} ${y2} ${mx + r} ${y2} H ${x2}`;
  }

  // Punti d'ingresso nel tachimetro totale: stesso ordine delle schede,
  // raccolti attorno al centro del quadrante ma sempre ben separati.
  function entryPoints(starts: { x: number; y: number }[], hub: { left: number; top: number; height: number }) {
    const n = starts.length;
    if (n === 0) return [];
    const gap = Math.min(34, (hub.height * 0.5) / Math.max(1, n - 1));
    const mid = Math.min(Math.max(starts.reduce((sum, s) => sum + s.y, 0) / n, hub.top + hub.height * 0.3), hub.top + hub.height * 0.62);
    return starts.map((_, i) => ({ x: hub.left, y: mid + (i - (n - 1) / 2) * gap }));
  }

  // Tutte le linee girano alla stessa x. Solo se due tratti verticali si
  // sovrapporrebbero, ognuna gira a una x sua, annidate così da non incrociarsi.
  function turnPoints(starts: { x: number; y: number }[], ends: { x: number; y: number }[]) {
    const left = Math.max(...starts.map((s) => s.x));
    const right = Math.min(...ends.map((e) => e.x));
    const shared = left + (right - left) * 0.5;
    const spans = starts.map((s, i) => [Math.min(s.y, ends[i].y), Math.max(s.y, ends[i].y)]);
    const clash = spans.some(([a1, b1], i) => spans.some(([a2, b2], j) => i < j && a1 < b2 + 6 && a2 < b1 + 6 && Math.abs(starts[i].y - ends[i].y) > 1 && Math.abs(starts[j].y - ends[j].y) > 1));
    if (!clash) return starts.map(() => shared);
    // chi deve scendere di più gira più vicino al totale (e viceversa per chi sale)
    const reach = starts.map((s, i) => ends[i].y - s.y);
    const order = reach.map((r, i) => ({ r, i })).sort((a, b) => Math.abs(a.r) - Math.abs(b.r));
    const step = (right - left) / (order.length + 1);
    const xs = new Array<number>(starts.length);
    order.forEach(({ i }, rank) => (xs[i] = left + step * (rank + 1)));
    return xs;
  }

  const flows = $derived.by((): Flow[] => {
    if (!geometry) return [];
    const { points, hub } = geometry;
    // ogni connessione misurata confluisce nel totale, anche se esclusa dal
    // bilanciamento: il suo traffico reale fa comunque parte della velocità
    const placed = feedLinks.flatMap((link) => {
      const start = points.get(link.key);
      return start ? [{ link, start }] : [];
    });
    const ends = entryPoints(placed.map((p) => p.start), hub);
    const turns = turnPoints(placed.map((p) => p.start), ends);
    return placed.map(({ link, start }, i) => {
      const end = ends[i];
      // tracciato squadrato: orizzontale → verticale → orizzontale
      const d = squarePath(start.x, start.y, end.x, end.y, turns[i]);
      const mbps = Math.max(0, live.get(link.key)?.mbps ?? 0);
      const idle = mbps < 0.5;
      // 1 Mbps ≈ 1,6 s per ciclo, 1 Gbps ≈ 0,35 s: scala logaritmica, mai sotto 0,3 s
      const duration = Math.max(0.3, 1.6 - Math.log10(Math.max(1, mbps)) * 0.42);
      return { key: link.key, d, color: linkColor(link.key), duration, idle, end };
    });
  });
</script>

<section class="system-panel" class:active class:error={status?.state === "error"} aria-labelledby="system-title">
  <div class="system-primary">
    <div class="system-copy">
      <span class="mode-kicker"><span class="mode-dot"></span> {t("Modalità sistema")}</span>
      <h1 id="system-title">{t("Un’unica Internet, dietro tutte le tue connessioni.")}</h1>
      <p>{t("Omni-Route distribuisce automaticamente i flussi di Windows tra Ethernet, Wi-Fi e tethering USB.")}</p>
    </div>

    <div class="master-control">
      <span class="control-state">{stateLabel()}</span>
      <button
        class="master-switch"
        class:on={status?.enabled}
        type="button"
        role="switch"
        aria-checked={status?.enabled ?? false}
        aria-label={t("Bilanciamento di sistema")}
        disabled={!status?.serviceAvailable || systemBalance.busy}
        onclick={() => void systemBalance.toggle(!(status?.enabled ?? false))}
      >
        <span></span>
      </button>
    </div>
  </div>

  {#if status && !status.serviceAvailable}
    <div class="system-error" role="alert">
      <Icon name="activity" size={17} />
      <div>
        <strong>{t("Il servizio Omni-Route è fermo.")}</strong>
        <span>{t("Le connessioni vengono rilevate dal servizio di sistema: finché è fermo l'elenco resta vuoto.")}</span>
        {#if systemBalance.startError}<span>{systemBalance.startError}</span>{/if}
      </div>
      <button class="service-start" type="button" onclick={() => void systemBalance.startService()} disabled={systemBalance.busy}>
        {t("Avvia servizio")}
      </button>
    </div>
  {:else if status?.lastError}
    <div class="system-error" role="alert">
      <Icon name="activity" size={17} />
      <div><strong>{errorTitle()}</strong><span>{status.lastError}</span></div>
    </div>
  {/if}

  <div class="stage" bind:this={stageEl} aria-label={t("Tachimetri modalità sistema")}>
    <div class="feeds">
      <div class="feeds-heading">
        <span>{t("Percorsi disponibili")}</span>
        <button type="button" onclick={() => void systemBalance.refresh()} disabled={systemBalance.busy}>
          <span class:spinning={systemBalance.busy}><Icon name="refresh" size={14} /></span>
          {t("Rileva ora")}
        </button>
      </div>
      {#if !status}
        <span class="link-placeholder">{t("Lettura del servizio…")}</span>
      {:else if !status.serviceAvailable}
        <span class="link-placeholder">{t("In attesa del servizio Omni-Route…")}</span>
      {:else if feedLinks.length === 0}
        <span class="link-placeholder">{t("Nessuna connessione Internet stabile disponibile.")}</span>
      {:else}
        {#if availableLinks.length === 0}
          <p class="off-hint">{disabledLinks.length === 1
            ? t("{name} è esclusa dal bilanciamento: accendila qui sotto per usarla con Omni-Route.", { name: disabledLinks[0].name })
            : t("Le tue connessioni sono escluse dal bilanciamento: accendile qui sotto per usarle con Omni-Route.")}</p>
        {/if}
        <div class="feed-list" style={`--feeds: ${feedLinks.length}`}>
          {#each feedLinks as link (link.key)}
            <div class="feed" class:off={!link.enabled} data-feed={link.key}>
              <MiniGauge
                row
                label={link.name}
                detail={link.enabled ? `${kindLabel(link.kind)} · ${link.localIp}` : `${kindLabel(link.kind)} · ${t("Esclusa")}`}
                value={meterValue(link.key)}
                average={live.get(link.key)?.avg ?? 0}
                averageLabel={meterAverageLabel(link.key)}
                {scaleMax}
                color={linkColor(link.key)}
                online={link.state !== "offline"}
              >
                <button
                  class="link-switch"
                  class:on={link.enabled}
                  type="button"
                  role="switch"
                  aria-label={t("Usa {name}", { name: link.name })}
                  aria-checked={link.enabled}
                  onclick={() => void systemBalance.setLinkEnabled(link.key, !link.enabled)}
                ><span></span></button>
              </MiniGauge>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    {#if geometry}
      <svg class="flows" width={geometry.width} height={geometry.height} aria-hidden="true">
        {#each flows as flow (flow.key)}
          <path class="flow-base" d={flow.d} style={`--flow-color: ${flow.color}`} />
          <path
            class="flow"
            class:idle={flow.idle}
            d={flow.d}
            style={`--flow-color: ${flow.color}; --flow-duration: ${flow.duration}s`}
          />
          <circle class="flow-sink" class:idle={flow.idle} cx={flow.end.x} cy={flow.end.y} r="4" style={`--flow-color: ${flow.color}`} />
        {/each}
      </svg>
    {/if}

    <div class="hub" bind:this={hubEl}>
      <MiniGauge
        label={t("Totale combinato")}
        value={totalNow}
        average={totalAvg}
        {scaleMax}
        color="var(--accent)"
        online={feedLinks.length > 0}
        featured
      />
    </div>
  </div>

  <div class="system-stats">
    <div class="system-metric featured">
      <span>{t("Traffico ora")}</span>
      <strong>{formatMbps(totalNow)}</strong>
    </div>
    <div class="system-metric">
      <span>{t("Media")}</span>
      <strong>{formatMbps(totalAvg)}</strong>
    </div>
    <div class="system-metric">
      <span>{t("Link disponibili")}</span>
      <strong>{availableLinks.length}</strong>
    </div>
    <div class="system-metric">
      <span>{t("Flussi vivi")}</span>
      <strong>{(status?.tcpFlows ?? 0) + (status?.udpFlows ?? 0)}</strong>
    </div>
  </div>

  <p class="system-note">
    {t("Bilanciamento automatico per flusso. I singoli download cifrati non vengono spezzati; per quelli usa l’acceleratore file qui sotto.")}
  </p>
</section>

<style>
  .system-panel {
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--accent) 18%, var(--border));
    border-radius: 18px;
    background:
      radial-gradient(ellipse 55% 60% at 0% 50%,
        color-mix(in srgb, var(--accent) 6%, transparent), transparent),
      var(--surface-1);
    box-shadow: var(--shadow-card);
    transition: border-color 220ms ease, box-shadow 220ms ease, background 220ms ease;
  }
  .system-panel.active {
    border-color: color-mix(in srgb, var(--success) 36%, var(--border));
    background:
      radial-gradient(ellipse 55% 60% at 0% 50%,
        color-mix(in srgb, var(--success) 5%, transparent), transparent),
      var(--surface-1);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--success) 10%, transparent),
                var(--shadow-card);
  }
  .system-panel.error { border-color: color-mix(in srgb, var(--error) 45%, var(--border)); }

  .system-primary {
    flex: none; display: flex; align-items: center;
    justify-content: space-between; gap: 1.5rem;
    padding: 0.9rem 1.2rem 0.6rem;
  }
  .system-copy { min-width: 0; max-width: 720px; }
  .mode-kicker {
    display: inline-flex; align-items: center; gap: 0.4rem;
    color: var(--text-subtle); font-size: 0.63rem; font-weight: 700;
    letter-spacing: 0.08em; text-transform: uppercase;
  }
  .mode-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--text-subtle);
    transition: background 300ms ease, box-shadow 300ms ease;
  }
  .active .mode-dot {
    background: var(--success);
    animation: mode-pulse 2.2s ease-in-out infinite;
  }
  @keyframes mode-pulse {
    0%, 100% { box-shadow: 0 0 0 2px color-mix(in srgb, var(--success) 22%, transparent); }
    50%       { box-shadow: 0 0 0 5px color-mix(in srgb, var(--success) 7%, transparent); }
  }
  h1 { margin: 0.25rem 0 0.22rem; font-size: clamp(1.15rem, 2.1vw, 1.55rem); line-height: 1.12; letter-spacing: -0.03em; }
  p { margin: 0; color: var(--text-muted); font-size: 0.8rem; line-height: 1.4; }

  /* ── Master switch ────────────────────────────────────────────────── */
  .master-control { flex: 0 0 auto; display: grid; justify-items: end; gap: 0.4rem; }
  .control-state { color: var(--text-muted); font-size: 0.73rem; font-weight: 650; }
  .master-switch, .link-switch {
    position: relative; border: 0; cursor: pointer;
    transition: background 180ms ease;
  }
  .master-switch {
    width: 58px; height: 32px; border-radius: 16px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  .master-switch span {
    position: absolute; top: 4px; left: 4px;
    width: 24px; height: 24px; border-radius: 50%;
    background: var(--text-muted);
    box-shadow: 0 2px 6px rgba(0,0,0,.3);
    transition: transform 200ms cubic-bezier(.2,.8,.2,1), background 180ms ease;
  }
  .master-switch.on {
    background: linear-gradient(135deg,
      color-mix(in srgb, var(--success) 80%, var(--accent) 20%),
      color-mix(in srgb, var(--success) 65%, var(--surface-1)));
    box-shadow: 0 0 10px color-mix(in srgb, var(--success) 20%, transparent);
  }
  .master-switch.on span { transform: translateX(26px); background: white; }
  .master-switch:disabled { opacity: 0.44; cursor: default; }

  /* ── Error banner ─────────────────────────────────────────────────── */
  .system-error {
    flex: none; display: flex; align-items: flex-start; gap: 0.6rem;
    margin: 0 1.2rem 0.55rem; padding: 0.55rem 0.8rem;
    border-radius: 9px; color: var(--error);
    background: color-mix(in srgb, var(--error) 8%, transparent);
    animation: fade-up 200ms ease;
  }
  @keyframes fade-up { from { opacity: 0; transform: translateY(4px); } }
  .system-error strong, .system-error span { display: block; }
  .system-error strong { font-size: 0.8rem; }
  .system-error span { margin-top: 0.15rem; color: var(--text-muted); font-size: 0.72rem; }
  .system-error > div { flex: 1 1 auto; min-width: 0; }
  .service-start {
    flex: none; align-self: center; padding: 0.3rem 0.72rem;
    border: 1px solid color-mix(in srgb, var(--error) 42%, transparent);
    border-radius: 7px; background: transparent;
    color: var(--error); font-size: 0.72rem; font-weight: 600; cursor: pointer;
    transition: background 120ms ease;
  }
  .service-start:hover:not(:disabled) { background: color-mix(in srgb, var(--error) 10%, transparent); }
  .service-start:disabled { opacity: 0.5; cursor: default; }

  /* ── Stage: feeds | flows | hub ───────────────────────────────────── */
  .stage {
    position: relative; flex: 1 1 auto; min-height: 0;
    display: grid;
    grid-template-columns: minmax(220px, 320px) minmax(48px, 1fr) auto;
    align-items: stretch; gap: 0;
    padding: 0.1rem 1.2rem 0.8rem;
  }
  .feeds {
    position: relative; z-index: 1; min-height: 0;
    display: flex; flex-direction: column; gap: 0.4rem; grid-column: 1;
  }
  .feeds-heading {
    flex: none; display: flex; align-items: center;
    justify-content: space-between; color: var(--text-subtle);
    font-size: 0.62rem; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase;
  }
  .feeds-heading button {
    display: inline-flex; align-items: center; gap: 0.28rem;
    border: 0; background: transparent; color: var(--text-muted);
    font-size: 0.68rem; text-transform: none; letter-spacing: 0; cursor: pointer;
    transition: color 120ms ease;
  }
  .feeds-heading button:hover { color: var(--text); }
  .feed-list {
    flex: 1 1 auto; min-height: 0;
    display: flex; flex-direction: column; justify-content: center; gap: 0.4rem;
  }
  .feed { flex: 0 1 82px; min-height: 46px; }
  .link-placeholder { padding: 0.9rem 0.2rem; color: var(--text-muted); font-size: 0.78rem; }
  .off-hint { flex: none; margin: 0.15rem 0 0; max-width: 36ch; color: var(--warning); font-size: 0.75rem; }
  .feed.off :global(.gauge) { border-style: dashed; }
  .feed.off :global(.gauge > :not(.link-switch)) { opacity: 0.68; }

  /* Hub (total gauge) */
  .hub { position: relative; z-index: 1; grid-column: 3; min-height: 0; height: 100%; aspect-ratio: 1 / 1.08; max-width: 46vw; justify-self: end; }
  .hub :global(.gauge) { padding: 0.35rem 0.45rem 0.55rem; }
  .hub :global(svg) { flex: 1 1 auto; min-height: 0; width: 100%; height: 100%; margin: 0 !important; }

  /* ── Flow lines ───────────────────────────────────────────────────── */
  .flows { position: absolute; inset: 0; z-index: 2; pointer-events: none; overflow: visible; }
  .flow-base { fill: none; stroke: var(--flow-color); stroke-width: 1.5; stroke-linejoin: round; opacity: 0.12; }
  .flow {
    fill: none; stroke: var(--flow-color); stroke-width: 2;
    stroke-linecap: round; stroke-linejoin: round; stroke-dasharray: 5 9;
    filter: drop-shadow(0 0 3px color-mix(in srgb, var(--flow-color) 45%, transparent));
    animation: flow var(--flow-duration, 1s) linear infinite;
  }
  .flow.idle { animation-play-state: paused; opacity: 0.28; filter: none; }
  .flow-sink {
    fill: var(--flow-color); stroke: var(--surface-1); stroke-width: 1.5;
    filter: drop-shadow(0 0 4px var(--flow-color));
    animation: sink-pulse 1.8s ease-in-out infinite;
    transform-box: fill-box; transform-origin: center;
  }
  .flow-sink.idle { animation: none; opacity: 0.4; filter: none; }
  @keyframes sink-pulse { 50% { transform: scale(1.4); } }
  @keyframes flow { to { stroke-dashoffset: -28; } }
  @media (prefers-reduced-motion: reduce) { .flow, .flow-sink { animation: none; } }

  /* ── Stats bar ────────────────────────────────────────────────────── */
  .system-stats {
    flex: none; display: grid; grid-template-columns: 1.3fr repeat(3, 1fr);
    border-block: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface-inset) 85%, transparent);
  }
  .system-metric { min-width: 0; padding: 0.5rem 1.05rem; border-right: 1px solid var(--border); }
  .system-metric:last-child { border-right: 0; }
  .system-metric span { display: block; margin-bottom: 0.12rem; color: var(--text-subtle); font-size: 0.63rem; }
  .system-metric strong { font-family: var(--font-numeric); font-size: 0.9rem; font-variant-numeric: tabular-nums lining-nums; }
  .system-metric.featured strong { color: var(--accent); font-size: 1rem; }

  /* Mini link toggle */
  .link-switch { flex: none; width: 34px; height: 19px; border-radius: 10px; background: var(--surface-3); }
  .link-switch span {
    position: absolute; top: 2.5px; left: 2.5px;
    width: 14px; height: 14px; border-radius: 50%;
    background: var(--text-subtle);
    transition: transform 160ms ease, background 160ms ease;
  }
  .link-switch.on {
    background: linear-gradient(135deg,
      color-mix(in srgb, var(--success) 80%, var(--accent) 20%),
      color-mix(in srgb, var(--success) 65%, var(--surface-1)));
  }
  .link-switch.on span { transform: translateX(15px); background: white; }

  .system-note { flex: none; max-width: none; padding: 0.42rem 1.2rem 0.52rem; color: var(--text-subtle); font-size: 0.65rem; }
  .spinning { display: inline-flex; animation: spin 800ms linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  @media (max-height: 700px) {
    .system-copy p, .system-note { display: none; }
    .system-primary { padding-block: 0.6rem 0.35rem; }
  }
  @media (max-width: 640px) {
    .system-primary { align-items: flex-start; padding: 0.9rem; }
    .stage { grid-template-columns: 1fr; grid-template-rows: minmax(170px, 1fr) auto; padding-inline: 0.9rem; gap: 0.55rem; }
    .hub { grid-column: 1; grid-row: 1; justify-self: center; max-width: none; }
    .feeds { grid-row: 2; }
    .flows { display: none; }
    .system-stats { grid-template-columns: repeat(2, 1fr); }
    .system-metric:nth-child(2) { border-right: 0; }
    .system-metric:nth-child(-n+2) { border-bottom: 1px solid var(--border); }
  }
</style>
