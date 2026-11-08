import type { ReactNode } from "react";
import { IconClose, IconSparkles } from "../icons";

type Props = {
  value: string;
  onChange: (value: string) => void;
  onClear: () => void;
  suggest: string[];
  suggestIndex: number;
  onPickSuggest: (name: string) => void;
  trailing?: ReactNode;
};

export function SearchBar({
  value,
  onChange,
  onClear,
  suggest,
  suggestIndex,
  onPickSuggest,
  trailing,
}: Props) {
  return (
    <div className="relative">
      <div className="flex items-center gap-2 rounded-md bg-surface-container-low px-space-sm py-1">
        <IconSparkles className="h-4 w-4 shrink-0 text-primary" />
        <label className="sr-only" htmlFor="q">
          Search clips
        </label>
        <input
          id="q"
          className="font-body-md min-w-0 flex-1 border-0 bg-transparent text-on-surface outline-none ring-0 focus:outline-none focus-visible:outline-none placeholder:text-outline placeholder:opacity-50 placeholder:text-xs"
          placeholder="Search clips, #tags, or time…"
          value={value}
          autoComplete="off"
          spellCheck={false}
          role="combobox"
          aria-expanded={suggest.length > 0}
          aria-controls="suggest"
          aria-autocomplete="list"
          onChange={(e) => onChange(e.target.value)}
        />
        {value ? (
          <button
            type="button"
            className="rounded-full p-0.5 text-on-surface-variant hover:bg-surface-container-high"
            aria-label="Clear search"
            onClick={onClear}
          >
            <IconClose className="h-3.5 w-3.5" />
          </button>
        ) : null}
        {trailing}
      </div>
      {suggest.length > 0 ? (
        <ul
          id="suggest"
          role="listbox"
          className="shadow-preview absolute z-20 mt-1 w-full overflow-hidden rounded-md border border-outline-variant/40 bg-surface-container-lowest"
        >
          {suggest.map((name, i) => (
            <li
              key={name}
              role="option"
              aria-selected={i === suggestIndex}
              className={`font-body-sm cursor-pointer px-space-sm py-space-xs ${
                i === suggestIndex
                  ? "bg-secondary-container text-on-secondary-container"
                  : ""
              }`}
              onMouseDown={(e) => {
                e.preventDefault();
                onPickSuggest(name);
              }}
            >
              #{name}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
