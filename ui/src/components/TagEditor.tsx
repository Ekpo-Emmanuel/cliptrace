import { useState } from "react";
import type { ClipCard, TagCount } from "../api";
import { IconClose, IconPlus } from "../icons";

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

  const kbd =
    "font-label-sm kbd-bevel inline-flex items-center gap-1 rounded-sm  bg-secondary-container px-1.5 py-0.5 text-on-surface cursor-pointer";

  return (
    <div className="font-label-sm flex flex-wrap items-center gap-1 text-on-surface-variant">
      {card.tags.map((name) => (
        <button
          key={name}
          type="button"
          className={kbd}
          aria-label={`Remove tag ${name}`}
          onClick={() => onRemove(name)}
        >
          {name}
          <IconClose className="h-3 w-3" />
        </button>
      ))}
      <input
        id="add-tag"
        className={`${kbd} w-24 border-0 outline-none ${open ? "" : "sr-only"}`}
        placeholder="Name"
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
          className={`${kbd} text-on-surface-variant`}
          onClick={() => document.getElementById("add-tag")?.focus()}
        >
          <IconPlus className="h-3 w-3" />
          Tag
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
