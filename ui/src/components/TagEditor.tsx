import { useState } from "react";
import type { ClipCard, TagCount } from "../api";

type Props = {
  card: ClipCard;
  allTags: TagCount[];
  onAdd: (tag: string) => void;
  onRemove: (tag: string) => void;
};

export function TagEditor({ card, allTags, onAdd, onRemove }: Props) {
  const [value, setValue] = useState("");
  const [open, setOpen] = useState(false);
  const full = card.tags.length >= 10;
  const suggestions = allTags
    .map((t) => t.name)
    .filter(
      (n) =>
        n.toLowerCase().includes(value.trim().toLowerCase()) &&
        !card.tags.some((t) => t.toLowerCase() === n.toLowerCase()),
    )
    .slice(0, 6);

  function commit() {
    const next = value.trim() || suggestions[0];
    if (next && !full) onAdd(next.slice(0, 32));
    setValue("");
    setOpen(false);
  }

  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5">
      {card.tags.map((name) => (
        <button
          key={name}
          type="button"
          className="tagchip group"
          aria-label={`Remove tag ${name}`}
          onClick={() => onRemove(name)}
        >
          #{name}
          <span
            aria-hidden="true"
            className="text-[color:var(--faint)] transition-colors group-hover:text-[color:var(--accent)] group-focus-visible:text-[color:var(--accent)]"
          >
            ×
          </span>
        </button>
      ))}
      <input
        id="add-tag"
        className={`w-24 border-0 border-b bg-transparent font-mono text-[10.5px] outline-none ${
          open ? "" : "sr-only"
        }`}
        style={{ borderColor: "var(--accent)", color: "var(--text)" }}
        placeholder="name"
        aria-label="Add tag"
        autoComplete="off"
        maxLength={32}
        value={value}
        list="tag-suggest"
        onFocus={() => setOpen(true)}
        onChange={(e) => setValue(e.target.value)}
        onBlur={() => {
          if (!value.trim()) setOpen(false);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            e.stopPropagation();
            commit();
          } else if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            setOpen(false);
            (document.getElementById("q") as HTMLInputElement | null)?.focus();
          }
        }}
      />
      {open || full ? null : (
        <button
          type="button"
          className="txt-link"
          onClick={() => document.getElementById("add-tag")?.focus()}
        >
          + tag
        </button>
      )}
      <datalist id="tag-suggest">
        {suggestions.map((n) => (
          <option key={n} value={n} />
        ))}
      </datalist>
    </div>
  );
}
