import type { ClipCard } from "../api";
import { kindMark } from "../lib/kind";
import { clock } from "../lib/time";

type Props = {
  card: ClipCard;
  selected: boolean;
  past: boolean;
  picked: boolean;
  pickMode: boolean;
  pending?: boolean;
  onSelect: () => void;
  onTogglePick: () => void;
};

function sourceName(exe: string) {
  return exe.replace(/\.exe$/i, "") || "unknown";
}

export function ClipRow({
  card,
  selected,
  past,
  picked,
  pickMode,
  pending = false,
  onSelect,
  onTogglePick,
}: Props) {
  const secret = card.kind === "secret";
  const mono = card.kind === "link" || card.kind === "code";
  const { code } = kindMark(card.kind);
  const tags = card.tags.slice(0, 2);
  const extra = card.tags.length - tags.length;
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
      data-pending={pending}
      className="trow"
      onMouseDown={(e) => {
        if ((e.target as HTMLElement).closest("button")) return;
        e.preventDefault();
        onSelect();
      }}
    >
      <span
        className="pt-4 pr-2 text-right font-mono text-[10px] tabular-nums"
        style={{ color: selected ? "var(--accent)" : "var(--faint)" }}
      >
        {clock(card.last_seen_at)}
      </span>

      <span className="tspine">
        {lit ? (
          <span
            className="tlit"
            style={{ height: selected && !past ? 21 : "100%" }}
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

      <div className="tbody min-w-0 px-3 py-2">
        {secret ? (
          <span className="hatch mt-1.5" />
        ) : (
          <p
            className={`truncate leading-[1.25] font-medium ${
              mono
                ? "font-mono text-[12px]"
                : "text-[14.5px] tracking-[-0.005em]"
            }`}
            style={{ color: "var(--text)" }}
          >
            {card.preview}
          </p>
        )}
        <p
          className="mt-1 flex items-center gap-1.5 truncate font-mono text-[9.5px]"
          style={{ color: "var(--faint)" }}
        >
          <span>
            {code} · {sourceName(card.source_exe)}
          </span>
          {tags.map((t) => (
            <span key={t}>#{t}</span>
          ))}
          {extra > 0 ? <span>+{extra}</span> : null}
          {card.pinned ? (
            <span style={{ color: "var(--accent)" }}>◆ pinned</span>
          ) : null}
        </p>
      </div>
    </div>
  );
}
