import { useEffect, useState } from "react";

export type Theme = "system" | "light" | "dark";

const KEY = "cliptrace-theme";

export function loadTheme(): Theme {
  try {
    const v = window.localStorage.getItem(KEY);
    if (v === "light" || v === "dark") return v;
  } catch {
    /* storage unavailable: follow the system */
  }
  return "system";
}

/** Apply and remember. "system" removes the override so Windows decides. */
export function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "system") root.removeAttribute("data-theme");
  else root.dataset.theme = theme;
  try {
    if (theme === "system") window.localStorage.removeItem(KEY);
    else window.localStorage.setItem(KEY, theme);
  } catch {
    /* ignore */
  }
}

/** The theme actually on screen: "system" resolves through Windows. */
export function useEffectiveTheme(theme: Theme): "light" | "dark" {
  const query = "(prefers-color-scheme: dark)";
  const [dark, setDark] = useState(() => window.matchMedia(query).matches);
  useEffect(() => {
    const mq = window.matchMedia(query);
    const on = () => setDark(mq.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, []);
  if (theme === "system") return dark ? "dark" : "light";
  return theme;
}
