import { useEffect, useRef } from "react";
import type { OpenCache } from "./useClips";

/**
 * The Tauri shell calls `window.cliptraceOnOpen` each time it shows the popup and
 * `window.cliptraceOnHide` just before it hides it. Registered once, on mount, so
 * the shell never finds them missing; they always call the latest handlers.
 */
export function usePopupLifecycle(
  onOpen: (cache?: OpenCache) => void,
  onHide: () => void,
) {
  const open = useRef(onOpen);
  const hide = useRef(onHide);
  open.current = onOpen;
  hide.current = onHide;

  useEffect(() => {
    window.cliptraceOnOpen = (cache) => open.current(cache);
    window.cliptraceOnHide = () => hide.current();
  }, []);
}
