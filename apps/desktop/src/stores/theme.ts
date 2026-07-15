import { create } from "zustand";
import { persist } from "zustand/middleware";

/** Appearance preference. Defaults to "system", as a native app should. */
export type ThemePreference = "light" | "dark" | "system";

/** The theme actually applied, once "system" has been resolved. */
export type ResolvedTheme = "light" | "dark";

interface ThemeState {
  preference: ThemePreference;
  setPreference: (preference: ThemePreference) => void;
}

const DARK_QUERY = "(prefers-color-scheme: dark)";

export const useThemeStore = create<ThemeState>()(
  persist(
    (set) => ({
      preference: "system",
      setPreference: (preference) => set({ preference }),
    }),
    { name: "audis.appearance" },
  ),
);

/** Resolve a preference against the OS setting. */
export function resolveTheme(preference: ThemePreference): ResolvedTheme {
  if (preference !== "system") return preference;
  return globalThis.matchMedia?.(DARK_QUERY).matches ? "dark" : "light";
}

/** Apply a theme by setting `data-theme`, which is what theme.css keys off. */
export function applyTheme(theme: ResolvedTheme): void {
  document.documentElement.dataset["theme"] = theme;
}

/**
 * Keep the document in sync with the store and the OS. Called once from
 * main.tsx rather than a component, so the theme never flickers on mount.
 * Returns an unsubscribe function.
 */
export function startThemeSync(): () => void {
  const apply = () => applyTheme(resolveTheme(useThemeStore.getState().preference));

  apply();

  const unsubscribeStore = useThemeStore.subscribe(apply);

  // Only relevant while the preference is "system", but subscribing
  // unconditionally keeps this branch-free and cheap.
  const media = globalThis.matchMedia?.(DARK_QUERY);
  media?.addEventListener("change", apply);

  return () => {
    unsubscribeStore();
    media?.removeEventListener("change", apply);
  };
}
