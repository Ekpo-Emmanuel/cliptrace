import { CopiedStrip, ErrorStrip, UndoStrip } from "./Strip";

type Props = {
  /** Clip waiting out its undo window, if any. */
  undoId: number | null;
  copied: boolean;
  error: string | null;
  onUndo: () => void;
  onDismissUndo: () => void;
  onDismissError: () => void;
};

/** The strips above the key hints. "Copied" yields to an error or an undo. */
export function ToastStack({
  undoId,
  copied,
  error,
  onUndo,
  onDismissUndo,
  onDismissError,
}: Props) {
  return (
    <>
      {undoId != null ? (
        <UndoStrip key={undoId} onUndo={onUndo} onDismiss={onDismissUndo} />
      ) : null}
      {copied && !error && undoId == null ? <CopiedStrip /> : null}
      {error ? <ErrorStrip message={error} onDismiss={onDismissError} /> : null}
    </>
  );
}
