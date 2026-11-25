<script lang="ts">
  import { lang, t } from "../lib/i18n.svelte";
  // Pannello di dettaglio di un link (§6.4): throughput medio/massimo,
  // errori, retry, colore, Rinomina/Cambia colore. È qui che vive il
  // gergo tecnico, non nella riga compatta.

  import type { LinkStats } from "../lib/ipc";
  import { renameLink, setLinkColor, forgetLink } from "../lib/ipc";
  import { formatMbps, formatBytes, formatRtt } from "../lib/format";
  import { untrack } from "svelte";
  import Icon from "./Icon.svelte";

  let { link, onClose }: { link: LinkStats; onClose: () => void } = $props();

  const PALETTE = ["#8b7cf8", "#d4954a", "#74b98a", "#c47ab5", "#6aaed4", "#d47a7a", "#7cb8d4", "#c4aa74"];

  let renaming = $state(false);
  let nameDraft = $state(untrack(() => link.name));

  async function saveName() {
    if (nameDraft.trim()) await renameLink(link.key, nameDraft.trim());
    renaming = false;
  }

  async function pickColor(color: string) {
    await setLinkColor(link.key, color);
  }

  async function onForget() {
    await forgetLink(link.key);
    onClose();
  }
</script>

<div class="overlay" onclick={onClose} role="presentation">
  <div
    class="card"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
    aria-label={t("Dettaglio {name}", { name: link.name })}
  >
    <div class="header">
      <span class="dot" style="--dot-color: {link.color}"></span>
      {#if renaming}
        <input
          bind:value={nameDraft}
          onkeydown={(e) => e.key === "Enter" && saveName()}
          onblur={saveName}
        />
      {:else}
        <button type="button" class="name-btn" onclick={() => (renaming = true)}>{link.name}</button>
      {/if}
      <button type="button" class="close" onclick={onClose} aria-label={t("Chiudi")}><Icon name="close" size={17} /></button>
    </div>

    <div class="colors">
      {#each PALETTE as c}
        <button
          type="button"
          class="swatch"
          class:active={c.toLowerCase() === link.color.toLowerCase()}
          style="--swatch-color: {c}"
          onclick={() => pickColor(c)}
          aria-label={t("Colore {c}", { c })}
        ></button>
      {/each}
    </div>

    <dl>
      <dt>{t("Velocità attuale")}</dt>
      <dd>{formatMbps(link.mbps)}</dd>
      <dt>{t("Media 10 s")}</dt>
      <dd>{formatMbps(link.avgMbps)}</dd>
      <dt>{t("Picco sessione")}</dt>
      <dd>{formatMbps(link.peakMbps)}</dd>
      {#if link.speedTestPending}
        <dt>{t("Ultimo speed test")}</dt>
        <dd>{t("Test velocità in corso…")}</dd>
      {:else if link.speedTestMbps != null}
        <dt>{t("Ultimo speed test")}</dt>
        <dd>{formatMbps(link.speedTestMbps)}</dd>
      {:else if link.speedTestAttempted}
        <dt>{t("Ultimo speed test")}</dt>
        <dd>{t("Test velocità non disponibile")}</dd>
      {/if}
      {#if link.rttMs != null}
        <dt>RTT</dt>
        <dd>{formatRtt(link.rttMs)}</dd>
      {/if}
      <dt>{t("Connessioni attive")}</dt>
      <dd>{link.connections}</dd>
      <dt>{t("Errori")}</dt>
      <dd>{link.errors}</dd>
      <dt>{t("Traffico totale")}</dt>
      <dd>{formatBytes(link.bytes)}</dd>
      <dt>{t("Chiave stabile")}</dt>
      <dd class="mono">{link.key}</dd>
    </dl>

    <button type="button" class="forget" onclick={onForget}>{t("Dimentica questo link")}</button>
  </div>
</div>

<style>
  .overlay {
    position: fixed; inset: 0;
    background: rgba(10, 9, 7, 0.65);
    backdrop-filter: blur(8px);
    display: flex; align-items: center; justify-content: center;
    z-index: 20;
    animation: overlay-in 160ms ease;
  }
  @keyframes overlay-in { from { opacity: 0; } }

  .card {
    background: var(--surface-1);
    border: 1px solid var(--border-strong);
    border-radius: 14px;
    padding: 1.15rem;
    width: min(348px, 90vw);
    display: flex; flex-direction: column; gap: 0.85rem;
    box-shadow: var(--shadow-float);
    animation: card-in 200ms cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes card-in { from { opacity: 0; transform: scale(0.96) translateY(6px); } }

  .header { display: flex; align-items: center; gap: 0.55rem; }
  .dot {
    width: 10px; height: 10px; border-radius: 50%;
    background: var(--dot-color); flex-shrink: 0;
    box-shadow: 0 0 6px color-mix(in srgb, var(--dot-color) 40%, transparent);
  }
  .name-btn {
    flex: 1; text-align: left; background: none; border: none;
    color: var(--text); font-size: 1rem; font-weight: 650; cursor: pointer; padding: 0;
    transition: color 120ms ease;
  }
  .name-btn:hover { color: var(--accent); }
  .header input {
    flex: 1; background: var(--surface-2);
    border: 1px solid color-mix(in srgb, var(--accent) 50%, transparent);
    border-radius: 7px; padding: 0.28rem 0.5rem;
    color: var(--text); font-size: 1rem;
    outline: none; transition: border-color 150ms ease;
  }
  .close {
    background: none; border: none; color: var(--text-muted); cursor: pointer;
    display: grid; place-items: center; border-radius: 6px;
    padding: 0.2rem; transition: color 120ms ease, background 120ms ease;
  }
  .close:hover { color: var(--text); background: var(--surface-2); }

  .colors { display: flex; gap: 0.38rem; flex-wrap: wrap; }
  .swatch {
    width: 20px; height: 20px; border-radius: 50%;
    background: var(--swatch-color); border: 2px solid transparent; cursor: pointer;
    transition: transform 120ms ease, border-color 120ms ease;
  }
  .swatch:hover { transform: scale(1.18); }
  .swatch.active { border-color: var(--text); transform: scale(1.12); }

  dl {
    display: grid; grid-template-columns: auto 1fr; gap: 0.3rem 0.75rem;
    margin: 0; font-size: 0.82rem;
  }
  dt { color: var(--text-muted); }
  dd { margin: 0; text-align: right; font-variant-numeric: tabular-nums; }
  .mono { font-family: ui-monospace, monospace; font-size: 0.72rem; word-break: break-all; }

  .forget {
    background: none;
    border: 1px solid color-mix(in srgb, var(--error) 25%, var(--border));
    color: var(--error); border-radius: 7px; padding: 0.38rem;
    cursor: pointer; font-size: 0.78rem;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .forget:hover { background: color-mix(in srgb, var(--error) 8%, transparent); border-color: color-mix(in srgb, var(--error) 45%, transparent); }
</style>
