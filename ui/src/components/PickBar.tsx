import { IconTrash } from "../icons";

type Props = {
  count: number;
  total: number;
  onSelectAll: () => void;
  onClear: () => void;
  onDelete: () => void;
};

export function PickBar({ count, total, onSelectAll, onClear, onDelete }: Props) {
  const allOn = count === total && total > 0;
  return (
    <div className="font-label-xs flex items-center gap-2 border-b border-outline-variant/30 bg-surface-container-low px-2.5 py-1.5 text-on-surface">
      <span className="tabular-nums">{count} selected</span>
      <button
        type="button"
        className="text-primary hover:underline disabled:opacity-40"
        disabled={allOn}
        onClick={onSelectAll}
      >
        Select all
      </button>
      <button
        type="button"
        className="text-on-surface-variant hover:underline"
        onClick={onClear}
      >
        Deselect all
      </button>
      <button
        type="button"
        className="ml-auto rounded p-1 text-error hover:bg-error-container"
        aria-label="Delete selected"
        onClick={onDelete}
      >
        <IconTrash className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
