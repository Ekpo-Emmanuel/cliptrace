import type { ClipCard } from "../api";
import { IconCheck, IconPin, IconTag } from "../icons";
import { kindMark } from "../lib/kind";
import { listDate } from "../lib/time";

type Props = {
  card: ClipCard;
  selected: boolean;
  picked: boolean;
  pickMode: boolean;
  onSelect: () => void;
  onTogglePick: () => void;
  onPin: () => void;
  onTag: () => void;
};

function sourceName(exe: string) {
  return exe.replace(/\.exe$/i, "") || "Unknown";
}

export function ClipRow({
  card,
  selected,
  picked,
  pickMode,
  onSelect,
  onTogglePick,
  onPin,
  onTag,
}: Props) {
  const secret = card.kind === "secret";
  const source = sourceName(card.source_exe);
  const { Icon: KindIcon, chip } = kindMark(card.kind);
  const tags = card.tags.slice(0, 2);
  const extra = card.tags.length - tags.length;

  return (
    <div
      role="option"
      aria-selected={selected}
      id={`clip-${card.id}`}
      className={`group relative flex cursor-pointer gap-2.5 px-2.5 py-2 ${
        selected
          ? "bg-surface-container-high/70"
          : "hover:bg-surface-container-low"
      }`}
      onMouseDown={(e) => {
        if ((e.target as HTMLElement).closest("button")) return;
        e.preventDefault();
        onSelect();
      }}
    >
      {selected ? (
        <span className="absolute bottom-1 right-0 top-1 w-[3px] rounded-l bg-primary" />
      ) : null}

      <button
        type="button"
        aria-label={picked ? "Unselect clip" : "Select clip"}
        aria-pressed={picked}
        className={`relative mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center ${
          picked || pickMode
            ? "bg-transparent"
            : `${chip} rounded-full group-hover:rounded-none group-hover:bg-transparent`
        }`}
        onMouseDown={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onTogglePick();
        }}
      >
        <KindIcon
          className={`h-3 w-3 ${
            picked || pickMode ? "opacity-0" : "group-hover:opacity-0"
          }`}
          strokeWidth={2.5}
        />
        <span
          className={`absolute inset-0 flex items-center justify-center ${
            picked || pickMode
              ? "opacity-100"
              : "opacity-0 group-hover:opacity-100"
          }`}
        >
          <span
            className={`flex h-3.5 w-3.5 items-center justify-center rounded-sm border-2 ${
              picked
                ? "border-primary bg-primary text-on-primary"
                : "border-on-surface-variant/70 bg-surface-container-lowest"
            }`}
          >
            {picked ? <IconCheck className="h-3 w-3" strokeWidth={2.5} /> : null}
          </span>
        </span>
      </button>

      <div className="min-w-0 flex-1">
        <span className="font-label-md block truncate text-on-surface">
          {source}
        </span>
        <p
          className={`font-body-sm mt-0.5 truncate ${
            secret ? "font-mono text-on-surface-variant" : "text-on-surface"
          }`}
        >
          {secret ? "••••••••••••" : card.preview}
        </p>
        {tags.length > 0 || card.pinned ? (
          <div className="mt-1 flex items-center gap-1">
            {tags.map((t) => (
              <span
                key={t}
                className="font-label-xs rounded bg-outline-variant/55 px-1.5 py-px text-on-surface"
              >
                {t}
              </span>
            ))}
            {extra > 0 ? (
              <span className="font-label-xs text-on-surface-variant/70">
                +{extra}
              </span>
            ) : null}
            {card.pinned ? <IconPin className="h-3 w-3 text-primary" /> : null}
          </div>
        ) : null}
      </div>

      <div className="relative w-16 shrink-0 self-start text-right">
        <span className="font-label-xs inline-block pt-0.5 tabular-nums text-on-surface-variant group-hover:opacity-0 group-focus-within:opacity-0">
          {listDate(card.last_seen_at)}
        </span>
        <div className="pointer-events-none absolute right-0 top-0 flex justify-end gap-0.5 opacity-0 group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100">
          {card.pinned ? null : (
            <button
              type="button"
              className="rounded p-1 text-on-surface-variant hover:bg-surface-container-highest hover:text-on-surface"
              aria-label="Pin"
              onClick={onPin}
            >
              <IconPin className="h-3.5 w-3.5" />
            </button>
          )}
          <button
            type="button"
            className="rounded p-1 text-on-surface-variant hover:bg-surface-container-highest hover:text-on-surface"
            aria-label="Add tag"
            onClick={onTag}
          >
            <IconTag className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    </div>
  );
}
