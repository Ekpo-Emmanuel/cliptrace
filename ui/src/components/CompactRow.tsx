import type { ClipCard } from "../api";
import { clock } from "../lib/time";

type Props = {
  card: ClipCard;
  selected: boolean;
  past: boolean;
  picked: boolean;
  pickMode: boolean;
  onSelect: () => void;
  onTogglePick: () => void;
};

/** Quick paste row: 46px, no time column. */
export function CompactRow({
  card,
  selected,
  past,
  picked,
  pickMode,
  onSelect,
  onTogglePick,
}: Props) {
  const secret = card.kind === "secret";
  const mono = card.kind === "link" || card.kind === "code";
  const lit = past || selected;

  let node = "tnode";
  if (pickMode) node += ` chk${picked ? " on" : ""}`;
  else if (selected) node += card.pinned ? " sel pin" : " sel";
  else if (card.pinned) node += " pin";
  if (!pickMode && past) node += " past";

  return (
    <div
      role="option"
      aria-selected={selected}
      id={`clip-${card.id}`}
      data-picked={picked}
      className="trow q"
      onMouseDown={(e) => {
        if ((e.target as HTMLElement).closest("button")) return;
        e.preventDefault();
        onSelect();
      }}
    >
      <span />
      <span className="tspine">
        {lit ? (
          <span
            className="tlit"
            style={{ height: selected && !past ? 23 : "100%" }}
          />
        ) : null}
        <button
          type="button"
          className={node}
          aria-label={picked ? "Unselect clip" : "Select clip"}
          aria-pressed={picked}
          onMouseDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onTogglePick();
          }}
        >
          {picked ? "✓" : null}
        </button>
      </span>
      <div className="tbody">
        <div className="flex min-w-0 flex-1 items-center pl-3">
        {secret ? (
          <span className="hatch" aria-label="Hidden secret" />
        ) : (
          <p
            className={`truncate leading-[1.25] font-medium ${
              mono
                ? "font-mono text-[12px]"
                : "text-[14.5px] tracking-[-0.005em]"
            }`}
          >
            {card.preview}
          </p>
        )}
        </div>
        <span
          className="flex items-center pr-3 pl-2 font-mono text-[10px] tabular-nums"
          style={{ color: card.pinned ? "var(--accent)" : "var(--faint)" }}
        >
          {card.pinned ? "◆" : clock(card.last_seen_at)}
        </span>
      </div>
    </div>
  );
}
