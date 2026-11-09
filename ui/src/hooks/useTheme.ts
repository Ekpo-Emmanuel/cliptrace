import { useEffect, useState } from "react";
import {
  applyTheme,
  loadTheme,
  useEffectiveTheme,
  type Theme,
} from "../lib/theme";

/** The chosen theme, applied to the document, plus the one actually on screen. */
export function useTheme() {
  const [theme, setTheme] = useState<Theme>(loadTheme);
  useEffect(() => applyTheme(theme), [theme]);
  const shown = useEffectiveTheme(theme);
  const toggle = () => setTheme(shown === "dark" ? "light" : "dark");
  return { theme, setTheme, shown, toggle };
}
