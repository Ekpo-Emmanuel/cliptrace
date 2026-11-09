import { useEffect, useState } from "react";
import { IconCheck, IconClose, IconTrash } from "../icons";

type UndoProps = {
  onUndo: () => void;
  onDismiss: () => void;
};

/** Undo strip: 5s countdown, draining bar. Parent removes the clip at 0. */
export function UndoStrip({ onUndo, onDismiss }: UndoProps) {
  const [left, setLeft] = useState(5);
  useEffect(() => {
    const t = window.setInterval(() => setLeft((n) => Math.max(0, n - 1)), 1000);
    return () => window.clearInterval(t);
  }, []);
  return (
    <div className="strip" role="status" aria-live="polite">
      <span className="strip-bar" />
      <IconTrash className="h-4 w-4 shrink-0" />
      <span className="flex-1 font-medium">Clip deleted</span>
      <span
        className="font-mono text-[11px]"
        style={{ color: "var(--accent)" }}
      >
        {left}s
      </span>
      <button type="button" className="txt-link" onClick={onUndo}>
        Undo
      </button>
      <button
        type="button"
        className="btn-ico !h-6 !w-6"
        aria-label="Dismiss"
        onClick={onDismiss}
      >
        <IconClose className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

/** Short confirmation shown after a copy. Dismisses itself via the parent timer. */
export function CopiedStrip() {
  return (
    <div className="strip ok" role="status" aria-live="polite">
      <IconCheck className="h-4 w-4 shrink-0" style={{ color: "var(--accent)" }} />
      <span className="min-w-0 flex-1 truncate font-medium">
        Copied to clipboard
      </span>
    </div>
  );
}

export function ErrorStrip({
  message,
  onDismiss,
}: {
  message: string;
  onDismiss: () => void;
}) {
  return (
    <div className="strip bad" role="alert">
      <span className="strip-bar" />
      <span style={{ color: "var(--bad)" }} aria-hidden>
        ⚠
      </span>
      <span className="min-w-0 flex-1 truncate font-semibold">{message}</span>
      <button
        type="button"
        className="btn-ico !h-6 !w-6"
        aria-label="Dismiss"
        onClick={onDismiss}
      >
        <IconClose className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
