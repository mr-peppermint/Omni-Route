// Store reattivo alimentato dallo `stats` event (§5 della spec). Un solo
// listener per l'intera app: `App.svelte` chiama `monitor.start()` una
// volta in `onMount`, ogni componente legge da qui.
//
// Estensione `.svelte.ts` (non `.ts`): richiesta da Svelte 5 per usare le
// rune (`$state`) fuori da un componente `.svelte`.

import {
  getSystemBalanceStatus,
  onStats,
  refreshSystemLinks,
  setSystemBalanceEnabled,
  setSystemLinkEnabled,
  setSystemLinkMetered,
  startSystemService,
  type StatsSnapshot,
  type SystemBalanceStatus,
} from "./ipc";

class MonitorStore {
  snapshot = $state<StatsSnapshot | null>(null);
  /** `true` dal primo evento `stats` ricevuto — prima di quello il
   * monitor non ha ancora mai risposto (avvio app, non un guasto). */
  connected = $state(false);
  private started = false;

  async start() {
    if (this.started) return;
    this.started = true;
    await onStats((s) => {
      this.snapshot = s;
      this.connected = true;
    });
  }
}

export const monitor = new MonitorStore();

class SystemBalanceStore {
  status = $state<SystemBalanceStatus | null>(null);
  busy = $state(false);
  private timer: ReturnType<typeof setInterval> | null = null;
  private consecutiveUnavailable = 0;

  start() {
    if (this.timer) return;
    void this.poll();
    this.timer = setInterval(() => void this.poll(), 1000);
  }

  stop() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
  }

  async poll() {
    try {
      const next = await getSystemBalanceStatus();
      if (!next.serviceAvailable && this.status?.serviceAvailable && this.consecutiveUnavailable < 2) {
        this.consecutiveUnavailable += 1;
        return;
      }
      this.consecutiveUnavailable = next.serviceAvailable ? 0 : this.consecutiveUnavailable + 1;
      this.status = next;
    } catch {
      this.consecutiveUnavailable += 1;
      if (this.consecutiveUnavailable >= 3) this.status = null;
    }
  }

  async toggle(enabled: boolean) {
    if (this.busy) return;
    this.busy = true;
    try {
      this.status = await setSystemBalanceEnabled(enabled);
    } finally {
      this.busy = false;
      await this.poll();
    }
  }

  startError = $state<string | null>(null);

  /** Starts the service if needed and persists the tunnel's enabled state. */
  async startTunnel() {
    if (this.busy) return;
    this.busy = true;
    this.startError = null;
    try {
      if (!this.status?.serviceAvailable) {
        await startSystemService();
        for (let attempt = 0; attempt < 10 && !this.status?.serviceAvailable; attempt += 1) {
          await new Promise((resolve) => setTimeout(resolve, 1000));
          await this.poll();
        }
      }
      if (!this.status?.serviceAvailable) {
        throw new Error(this.status?.lastError ?? "The Omni-Route service did not become available.");
      }
      this.status = await setSystemBalanceEnabled(true);
    } catch (error) {
      this.startError = String(error);
    } finally {
      this.busy = false;
      await this.poll();
    }
  }

  async refresh() {
    if (this.busy) return;
    this.busy = true;
    try {
      this.status = await refreshSystemLinks();
    } catch {
      await this.poll();
    } finally {
      this.busy = false;
    }
  }

  async setLinkEnabled(key: string, enabled: boolean) {
    this.status = await setSystemLinkEnabled(key, enabled);
  }

  async setLinkMetered(key: string, metered: boolean) {
    this.status = await setSystemLinkMetered(key, metered);
  }
}

export const systemBalance = new SystemBalanceStore();
