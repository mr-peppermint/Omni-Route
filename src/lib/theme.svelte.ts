// Theme management (system / light / dark)
export type ThemePreference = "system" | "light" | "dark";

const STORAGE_KEY = "omni-route-theme";

class ThemeStore {
  preference = $state<ThemePreference>(readStored());

  isDark = $derived.by(() => {
    if (this.preference === "dark") return true;
    if (this.preference === "light") return false;
    if (typeof window !== "undefined" && window.matchMedia) {
      return window.matchMedia("(prefers-color-scheme: dark)").matches;
    }
    return true;
  });

  constructor() {
    $effect.root(() => {
      $effect(() => {
        apply(this.preference);
        if (typeof localStorage !== "undefined") {
          localStorage.setItem(STORAGE_KEY, this.preference);
        }
      });
    });
  }

  set(pref: ThemePreference) {
    this.preference = pref;
  }

  toggle() {
    this.set(this.isDark ? "light" : "dark");
  }
}

function readStored(): ThemePreference {
  if (typeof localStorage === "undefined") return "dark";
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored === "light" || stored === "dark" || stored === "system" ? stored : "dark";
}

function apply(pref: ThemePreference) {
  if (typeof document === "undefined") return;
  if (pref === "system") {
    delete document.documentElement.dataset.theme;
  } else {
    document.documentElement.dataset.theme = pref;
  }
}

export const theme = new ThemeStore();
