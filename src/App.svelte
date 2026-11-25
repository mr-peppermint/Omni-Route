<script lang="ts">
  import { lang, t } from "./lib/i18n.svelte";
  import { onMount } from "svelte";
  import "./app.css";
  import { monitor, systemBalance } from "./lib/stores.svelte";
  import { doctorReport } from "./lib/ipc";
  import { formatMbps } from "./lib/format";
  import { theme } from "./lib/theme.svelte";
  import "./lib/accent.svelte";
  import Gauge from "./components/Gauge.svelte";
  import LinkChip from "./components/LinkChip.svelte";
  import LinkCard from "./components/LinkCard.svelte";
  import BandwidthChart from "./components/BandwidthChart.svelte";
  import DownloadPanel from "./components/DownloadPanel.svelte";
  import UrlBar from "./components/UrlBar.svelte";
  import Settings from "./components/Settings.svelte";
  import SpeedTest from "./components/SpeedTest.svelte";
  import Icon from "./components/Icon.svelte";
  import SystemBalancePanel from "./components/SystemBalancePanel.svelte";

  onMount(() => {
    monitor.start();
    systemBalance.start();
    return () => systemBalance.stop();
  });

  let selectedLinkKey = $state<string | null>(null);
  let settingsOpen = $state(false);
  let speedTestOpen = $state(false);
  let showComposer = $state(false);
  let acceleratorOpen = $state(false);
  let scanning = $state(false);
  let scanMessage = $state<string | null>(null);
  let scanMessageTimer: ReturnType<typeof setTimeout> | null = null;

  const snapshot = $derived(monitor.snapshot);
  const links = $derived(snapshot?.links ?? []);
  const liveLinks = $derived(links.filter((link) => link.state !== "offline"));
  const onlineLinks = $derived(liveLinks.filter((link) => link.enabled && link.state !== "disabled"));
  const offlineLinks = $derived(links.filter((link) => link.state === "offline"));
  const selectedLink = $derived(links.find((link) => link.key === selectedLinkKey) ?? null);
  const download = $derived(snapshot?.download ?? null);
  const terminalDownload = $derived(
    download != null && ["done", "error", "cancelled"].includes(download.state),
  );

  $effect(() => {
    if (!download) showComposer = false;
    if (download && ["running", "paused", "verifying"].includes(download.state)) showComposer = false;
  });

  async function scanNow() {
    if (scanning) return;
    scanning = true;
    scanMessage = null;
    try {
      const report = await doctorReport();
      const count = report.links.length;
      scanMessage = count === 1 ? t("1 connessione verificata") : t("{n} connessioni verificate", { n: count });
    } catch {
      scanMessage = t("Scansione non riuscita");
    } finally {
      scanning = false;
      if (scanMessageTimer) clearTimeout(scanMessageTimer);
      scanMessageTimer = setTimeout(() => (scanMessage = null), 4500);
    }
  }
</script>

<main class="shell">
  <header class="topbar">
    <div class="brand-lockup">
      <!-- Two links converging into one connection -->
      <span class="brand-mark" class:connected={monitor.connected}>
        <img src="/brand-mark.svg" alt="" />
        <span class="brand-pulse"></span>
      </span>
      <div>
        <span class="brand">Omni-Route</span>
        <span class="brand-subtitle">{t("Internet multi-link per Windows")}</span>
      </div>
    </div>

    <div class="top-actions">
      <span class="monitor-state" class:ready={monitor.connected}>
        <span class="status-dot"></span>
        <span class="state-text">{monitor.connected ? t("Monitor attivo") : t("Connessione al motore…")}</span>
      </span>
      <button
        class="icon-button theme-toggle"
        type="button"
        onclick={() => theme.toggle()}
        title={theme.isDark ? "Switch to light theme" : "Switch to dark theme"}
        aria-label={theme.isDark ? "Switch to light theme" : "Switch to dark theme"}
      >
        <span class="theme-icon" class:is-sun={!theme.isDark}>
          <Icon name={theme.isDark ? "moon" : "sun"} size={16} />
        </span>
      </button>
      <button class="button quiet" type="button" onclick={() => (speedTestOpen = true)}>
        <Icon name="activity" size={15} /> {t("Diagnostica")}
      </button>
      <button class="icon-button" type="button" onclick={() => (settingsOpen = true)} aria-label={t("Impostazioni")}>
        <Icon name="settings" size={17} />
      </button>
    </div>
  </header>

  <div class="workspace">
    <SystemBalancePanel />

    <button class="accelerator" type="button" onclick={() => (acceleratorOpen = true)} aria-haspopup="dialog">
      <span class="accelerator-icon">
        <Icon name="download" size={16} />
      </span>
      <span class="accelerator-text">
        <strong>{t("Acceleratore per singoli file")}</strong>
        <small>{t("Somma i link quando il server consente di dividere il download.")}</small>
      </span>
      <span class="accelerator-arrow">
        <Icon name="chevron-right" size={16} />
      </span>
    </button>
  </div>
</main>

{#if acceleratorOpen}
  <div class="sheet-overlay" onclick={() => (acceleratorOpen = false)} role="presentation">
    <div
      class="sheet"
      role="dialog"
      aria-modal="true"
      aria-labelledby="accelerator-title"
      tabindex="-1"
      onclick={(event) => event.stopPropagation()}
      onkeydown={(event) => event.stopPropagation()}
    >
      <header class="sheet-head">
        <span class="accelerator-icon"><Icon name="download" size={17} /></span>
        <div><strong id="accelerator-title">{t("Acceleratore per singoli file")}</strong><small>{t("Somma i link quando il server consente di dividere il download.")}</small></div>
        <button class="icon-button" type="button" onclick={() => (acceleratorOpen = false)} aria-label={t("Chiudi")}><Icon name="close" size={18} /></button>
      </header>
      <div class="accelerator-body">
    <section class="hero-grid" aria-label={t("Riepilogo velocità")}>
      <article class="card gauge-card">
        <div class="section-heading">
          <div>
            <p class="eyebrow">{t("Traffico combinato")}</p>
            <h1>{t("Velocità in tempo reale")}</h1>
          </div>
          <span class="live-pill"><span></span> {t("Live")}</span>
        </div>
        <Gauge
          links={liveLinks}
          totalMbps={snapshot?.totalMbps ?? 0}
          totalAvgMbps={snapshot?.totalAvgMbps ?? 0}
          scaleMax={snapshot?.scaleMax ?? 10}
          gainFactor={snapshot?.gainFactor ?? 1}
        />
        <div class="metric-strip">
          <div class="metric">
            <span class="metric-label">{t("Media 10 s")}</span>
            <strong>{formatMbps(snapshot?.totalAvgMbps ?? 0)}</strong>
          </div>
          <div class="metric">
            <span class="metric-label">{t("Connessioni vive")}</span>
            <strong>{onlineLinks.length}</strong>
          </div>
          <div class="metric">
            <span class="metric-label">{t("Aggregazione")}</span>
            <strong>{onlineLinks.length > 1 ? `${(snapshot?.gainFactor ?? 1).toLocaleString(lang.locale, { maximumFractionDigits: 1 })}×` : t("Singola")}</strong>
          </div>
        </div>
      </article>

      <article class="card connections-card">
        <div class="section-heading connection-heading">
          <div>
            <p class="eyebrow">{t("Percorsi Internet")}</p>
            <h2>{t("Connessioni vive")}</h2>
          </div>
          <button class="button quiet scan-button" type="button" onclick={scanNow} disabled={scanning}>
            <span class:spinning={scanning}><Icon name="refresh" size={16} /></span>
            {scanning ? t("Verifica…") : t("Rileva ora")}
          </button>
        </div>

        {#if scanMessage}
          <div class="scan-result" role="status"><Icon name="check" size={15} /> {scanMessage}</div>
        {/if}

        <div class="connection-list">
          {#if !monitor.connected}
            <div class="empty-state">
              <span class="empty-icon"><Icon name="wifi" size={24} /></span>
              <strong>{t("Avvio del monitor")}</strong>
              <p>{t("Sto leggendo le interfacce di rete di Windows.")}</p>
            </div>
          {:else if liveLinks.length === 0}
            <div class="empty-state">
              <span class="empty-icon"><Icon name="wifi" size={24} /></span>
              <strong>{t("Nessuna connessione viva")}</strong>
              <p>{t("Collega Wi-Fi, Ethernet o tethering USB: apparirà automaticamente.")}</p>
              <button class="button secondary" type="button" onclick={scanNow}>{t("Rileva di nuovo")}</button>
            </div>
          {:else}
            {#each liveLinks as link (link.key)}
              <LinkChip {link} onSelect={(key) => (selectedLinkKey = key)} />
            {/each}
          {/if}
        </div>

        {#if offlineLinks.length > 0}
          <details class="remembered">
            <summary>{offlineLinks.length} {offlineLinks.length === 1 ? t("connessione memorizzata") : t("connessioni memorizzate")}</summary>
            <div class="remembered-list">
              {#each offlineLinks as link (link.key)}
                <LinkChip {link} onSelect={(key) => (selectedLinkKey = key)} />
              {/each}
            </div>
          </details>
        {/if}
      </article>
    </section>

    <section class="card chart-card">
      <div class="section-heading compact">
        <div>
          <p class="eyebrow">{t("Andamento")}</p>
          <h2>{t("Velocità combinata")}</h2>
        </div>
        <span class="chart-caption">{t("Valori smussati, aggiornamento 4 Hz")}</span>
      </div>
      <BandwidthChart links={liveLinks} totalMbps={snapshot?.totalMbps ?? 0} ts={snapshot?.ts ?? 0} />
    </section>

    <section class="card task-card" aria-label={t("Download")}>
      {#if download && !showComposer}
        <DownloadPanel
          {download}
          totalMbps={snapshot?.totalMbps ?? 0}
          totalAvgMbps={snapshot?.totalAvgMbps ?? 0}
          onNewDownload={() => (showComposer = true)}
        />
      {:else}
        <UrlBar onStarted={() => (showComposer = false)} />
        {#if terminalDownload}
          <button class="back-to-result" type="button" onclick={() => (showComposer = false)}>{t("Torna all’ultimo risultato")}</button>
        {/if}
      {/if}
    </section>
      </div>
    </div>
  </div>
{/if}

<svelte:window onkeydown={(event) => { if (event.key === "Escape" && acceleratorOpen && !settingsOpen && !speedTestOpen && !selectedLink) acceleratorOpen = false; }} />

{#if selectedLink}
  {#key selectedLink.key}
    <LinkCard link={selectedLink} onClose={() => (selectedLinkKey = null)} />
  {/key}
{/if}

{#if settingsOpen}
  <Settings onClose={() => (settingsOpen = false)} />
{/if}

{#if speedTestOpen}
  <SpeedTest onClose={() => (speedTestOpen = false)} />
{/if}

<style>
  /* ── Shell layout ─────────────────────────────────────────────────── */
  .shell {
    width: min(1180px, 100%);
    height: 100vh;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* ── Top bar ──────────────────────────────────────────────────────── */
  .topbar {
    flex: none;
    min-height: 52px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0 1.4rem;
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg) 82%, transparent);
    backdrop-filter: blur(20px) saturate(1.4);
    position: relative;
    z-index: 20;
  }

  /* Brand */
  .brand-lockup { display: flex; align-items: center; gap: 0.65rem; }
  .brand-mark {
    position: relative;
    width: 32px; height: 32px;
    display: grid; place-items: center;
    border-radius: 9px;
    overflow: visible;
    transition: box-shadow 300ms ease, border-color 300ms ease;
  }
  .brand-mark img { display: block; width: 100%; height: 100%; border-radius: inherit; }
  .brand-mark.connected {
    border-color: color-mix(in srgb, var(--success) 40%, transparent);
    box-shadow: 0 0 10px color-mix(in srgb, var(--success) 14%, transparent);
  }
  .brand-pulse {
    position: absolute; inset: -4px;
    border-radius: 12px;
    border: 1px solid color-mix(in srgb, var(--accent) 28%, transparent);
    animation: brand-breathe 3s ease-in-out infinite;
    pointer-events: none;
  }
  @keyframes brand-breathe {
    0%, 100% { opacity: 0; transform: scale(1); }
    50%       { opacity: 1; transform: scale(1.08); }
  }
  .brand { display: block; font-size: 0.95rem; font-weight: 700; letter-spacing: -0.025em; }
  .brand-subtitle { display: block; margin-top: 0.04rem; color: var(--text-subtle); font-size: 0.68rem; }

  /* Actions */
  .top-actions { display: flex; align-items: center; gap: 0.45rem; }
  .monitor-state {
    display: flex; align-items: center; gap: 0.35rem;
    margin-right: 0.18rem; color: var(--text-muted); font-size: 0.73rem;
  }
  .status-dot {
    width: 6px; height: 6px; border-radius: 50%;
    background: var(--warning);
    transition: background 400ms ease, box-shadow 400ms ease;
  }
  .monitor-state.ready .status-dot {
    background: var(--success);
    animation: dot-pulse 2.2s ease-in-out infinite;
  }
  @keyframes dot-pulse {
    0%, 100% { box-shadow: 0 0 0 2px color-mix(in srgb, var(--success) 20%, transparent); }
    50%       { box-shadow: 0 0 0 5px color-mix(in srgb, var(--success) 7%, transparent); }
  }
  .state-text { display: none; }
  @media (min-width: 760px) { .state-text { display: inline; } }

  

  /* ── Workspace ────────────────────────────────────────────────────── */
  .workspace {
    flex: 1 1 auto; min-height: 0;
    padding: 0.8rem 1.4rem 0.9rem;
    display: flex; flex-direction: column; gap: 0.65rem; overflow: auto;
  }
  .workspace > :global(.system-panel) { flex: 1 1 auto; }

  /* ── Accelerator button ───────────────────────────────────────────── */
  .accelerator {
    flex: none; width: 100%; color: inherit; font: inherit; text-align: left;
    display: grid; grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center; gap: 0.7rem;
    min-height: 50px; padding: 0.5rem 0.85rem;
    border: 1px solid var(--border); border-radius: var(--radius-lg);
    background: var(--surface-1); box-shadow: var(--shadow-card); cursor: pointer;
    transition: border-color 150ms ease, background 150ms ease, transform 120ms ease;
  }
  .accelerator:hover {
    border-color: color-mix(in srgb, var(--accent) 25%, var(--border));
    background: var(--surface-2); transform: translateY(-1px);
  }
  .accelerator-icon {
    width: 34px; height: 34px; display: grid; place-items: center;
    border-radius: 8px; color: var(--accent-2);
    background: color-mix(in srgb, var(--accent-2) 10%, var(--surface-2));
  }
  .accelerator-text strong { display: block; font-size: 0.84rem; }
  .accelerator-text small { display: block; margin-top: 0.16rem; color: var(--text-subtle); font-size: 0.68rem; font-weight: 400; }
  .accelerator-arrow { color: var(--text-subtle); transition: transform 150ms ease; }
  .accelerator:hover .accelerator-arrow { transform: translateX(3px); }

  /* ── Accelerator sheet ────────────────────────────────────────────── */
  .sheet-overlay {
    position: fixed; inset: 0; z-index: 40;
    display: grid; place-items: center; padding: 1rem;
    background: rgba(10, 9, 7, 0.72);
    backdrop-filter: blur(10px);
    animation: overlay-in 180ms ease;
  }
  @keyframes overlay-in { from { opacity: 0; } }
  .sheet {
    width: min(1120px, 100%); max-height: calc(100vh - 2rem);
    display: flex; flex-direction: column; overflow: hidden;
    border: 1px solid var(--border-strong); border-radius: 18px;
    background: var(--bg); box-shadow: var(--shadow-float);
    animation: sheet-in 220ms cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes sheet-in { from { opacity: 0; transform: scale(0.97) translateY(8px); } }
  .sheet-head {
    flex: none; display: grid; grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center; gap: 0.75rem; padding: 0.7rem 0.9rem;
    border-bottom: 1px solid var(--border); background: var(--surface-1);
  }
  .sheet-head strong, .sheet-head small { display: block; }
  .sheet-head strong { font-size: 0.86rem; }
  .sheet-head small { margin-top: 0.16rem; color: var(--text-subtle); font-size: 0.68rem; font-weight: 400; }
  .accelerator-body {
    min-height: 0; overflow: auto;
    display: grid; grid-auto-rows: max-content; align-content: start; gap: 1rem; padding: 1rem;
  }

  /* ── Cards ────────────────────────────────────────────────────────── */
  .hero-grid { display: grid; grid-template-columns: minmax(0, 1.12fr) minmax(300px, 0.88fr); gap: 1rem; }
  .card { background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-lg); box-shadow: var(--shadow-card); }
  .gauge-card { padding: 1.1rem 1.2rem 0; overflow: hidden; }
  .connections-card { padding: 1.1rem 1rem 0.75rem; min-height: 420px; }
  .section-heading { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
  .section-heading.compact { padding: 1.05rem 1.15rem 0; }
  .section-heading h1, .section-heading h2 { margin: 0.14rem 0 0; line-height: 1.18; letter-spacing: -0.025em; }
  .section-heading h1 { font-size: clamp(1.2rem, 2vw, 1.48rem); }
  .section-heading h2 { font-size: 1.06rem; }
  .eyebrow { margin: 0; color: var(--text-subtle); font-size: 0.63rem; font-weight: 700; letter-spacing: 0.09em; text-transform: uppercase; }
  .live-pill { display: inline-flex; align-items: center; gap: 0.4rem; color: var(--success); font-size: 0.72rem; font-weight: 650; }
  .live-pill span { width: 6px; height: 6px; border-radius: 50%; background: currentColor; animation: dot-pulse 2.2s ease-in-out infinite; }

  .gauge-card :global(svg) { width: min(300px, 75%); height: auto; margin: -0.2rem auto -0.25rem; }
  .metric-strip { display: grid; grid-template-columns: repeat(3, 1fr); margin: 0 -1.2rem; border-top: 1px solid var(--border); background: var(--surface-inset); }
  .metric { min-width: 0; padding: 0.75rem 0.9rem; border-right: 1px solid var(--border); }
  .metric:last-child { border-right: 0; }
  .metric-label { display: block; margin-bottom: 0.2rem; color: var(--text-subtle); font-size: 0.65rem; }
  .metric strong { font-family: var(--font-numeric); font-size: 0.9rem; font-variant-numeric: tabular-nums lining-nums; }

  .connection-heading { padding: 0 0.2rem 0.72rem; }
  .connection-list { display: grid; gap: 0.38rem; }
  .scan-result {
    display: flex; align-items: center; gap: 0.38rem;
    margin: 0 0.2rem 0.6rem; padding: 0.42rem 0.58rem;
    border-radius: 8px; color: var(--success);
    background: color-mix(in srgb, var(--success) 8%, transparent); font-size: 0.74rem;
    animation: fade-up 200ms ease;
  }
  @keyframes fade-up { from { opacity: 0; transform: translateY(4px); } }
  .spinning { display: inline-flex; animation: spin 0.8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .empty-state {
    min-height: 220px; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    text-align: center; padding: 1.5rem; color: var(--text-muted);
  }
  .empty-state strong { margin-top: 0.75rem; color: var(--text); font-size: 0.95rem; }
  .empty-state p { max-width: 32ch; margin: 0.35rem 0 0.9rem; font-size: 0.82rem; line-height: 1.42; }
  .empty-icon { width: 42px; height: 42px; display: grid; place-items: center; border-radius: 50%; color: var(--text-subtle); background: var(--surface-2); }
  .remembered { margin-top: 0.65rem; padding-top: 0.6rem; border-top: 1px solid var(--border); }
  .remembered summary { padding: 0.28rem 0.38rem; color: var(--text-subtle); cursor: pointer; font-size: 0.72rem; }
  .remembered-list { display: grid; gap: 0.3rem; margin-top: 0.3rem; opacity: 0.8; }

  .chart-card { overflow: hidden; }
  .chart-caption { color: var(--text-subtle); font-size: 0.7rem; }
  .chart-card :global(.chart-wrap) { padding: 0.25rem 1rem 0.75rem; }
  .task-card { overflow: hidden; border-color: color-mix(in srgb, var(--accent) 16%, var(--border)); }
  .back-to-result { display: block; margin: -0.3rem auto 0.75rem; border: 0; background: transparent; color: var(--text-muted); font-size: 0.72rem; cursor: pointer; }

  @media (max-width: 880px) {
    .hero-grid { grid-template-columns: 1fr; }
    .connections-card { min-height: auto; }
    .topbar { position: static; }
  }
  @media (max-width: 640px) {
    .topbar { padding: 0 1rem; }
    .brand-subtitle, .top-actions .quiet { display: none; }
    .workspace { padding: 0.8rem 1rem; }
    .gauge-card { padding-inline: 0.9rem; }
    .metric-strip { margin-inline: -0.9rem; }
    .metric { padding-inline: 0.65rem; }
    .metric-label { min-height: 2em; }
    .chart-caption { display: none; }
    .connection-heading { align-items: flex-start; }
  }
</style>
