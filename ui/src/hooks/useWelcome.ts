import { useCallback, useEffect, useState } from "react";
import type { CliptraceApi } from "../api";
import type { Toasts } from "./useToasts";

/** First-run welcome screen: opens until dismissed once, and can be shown again. */
export function useWelcome(api: CliptraceApi | null, toasts: Toasts) {
  const [open, setOpen] = useState(false);
  const [hotkey, setHotkey] = useState("Ctrl+Alt+V");

  useEffect(() => {
    if (!api) return;
    void api
      .getSettings()
      .then((s) => {
        setHotkey(s.hotkey);
        if (!s.welcomed) setOpen(true);
      })
      .catch(() => {});
  }, [api]);

  const show = useCallback(() => setOpen(true), []);

  const finish = useCallback(() => {
    setOpen(false);
    window.setTimeout(() => document.getElementById("q")?.focus(), 0);
    api
      ?.updateSettings({ welcomed: true })
      .catch(() =>
        toasts.showError("Could not save that you have seen the welcome", false),
      );
  }, [api, toasts]);

  return { open, hotkey, show, finish };
}

export type WelcomeState = ReturnType<typeof useWelcome>;
