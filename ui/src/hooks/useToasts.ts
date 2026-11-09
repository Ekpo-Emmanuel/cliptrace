import { useCallback, useRef, useState } from "react";

const COPIED_MS = 1800;
const ERROR_MS = 2400;

/** State for the transient "copied" and error strips at the bottom of the popup. */
export function useToasts() {
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const copiedTimer = useRef(0);
  const errorTimer = useRef(0);

  const flashCopied = useCallback(() => {
    setCopied(true);
    window.clearTimeout(copiedTimer.current);
    copiedTimer.current = window.setTimeout(() => setCopied(false), COPIED_MS);
  }, []);

  /** Show an error. With `autoDismiss` off it stays until the user closes it. */
  const showError = useCallback((message: string, autoDismiss = true) => {
    window.clearTimeout(errorTimer.current);
    setError(message);
    if (autoDismiss) {
      errorTimer.current = window.setTimeout(() => setError(null), ERROR_MS);
    }
  }, []);

  const clearError = useCallback(() => {
    window.clearTimeout(errorTimer.current);
    setError(null);
  }, []);

  return { copied, error, flashCopied, showError, clearError };
}

export type Toasts = ReturnType<typeof useToasts>;
