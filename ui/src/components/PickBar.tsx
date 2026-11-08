import { IconClose } from "../icons";

type Props = {
  count: number;
  total: number;
  onSelectAll: () => void;
  onClear: () => void;
  onDelete: () => void;
};

/** Selection bar. Sits over the header while clips are checked. */
export function PickBar({ count, total, onSelectAll, onClear, onDelete }: Props) {
  const allOn = count === total && total > 0;
  return (
    <div className="bulkbar">
      <button
        type="button"
        className="btn-ico"
        aria-label="Clear selection"
        onClick={onClear}
      >
        <IconClose className="h-4 w-4" />
      </button>
      <span
        className="text-[22px] leading-none font-extrabold tabular-nums"
        style={{ color: "var(--accent)" }}
      >
        {count}
      </span>
      <span className="font-mono text-[10.5px]" style={{ color: "var(--muted)" }}>
        selected of {total}
      </span>
      <button
        type="button"
        className="txt-link disabled:opacity-40"
        disabled={allOn}
        onClick={onSelectAll}
      >
        select all
      </button>
      <button type="button" className="txt-link" onClick={onClear}>
        none
      </button>
      <button type="button" className="btn-bad ml-auto" onClick={onDelete}>
        Delete {count}
      </button>
    </div>
  );
}
