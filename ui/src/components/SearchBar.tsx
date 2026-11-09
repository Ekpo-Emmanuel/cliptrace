import type { ReactNode } from "react";
import { IconClose, IconCollapse, IconExpand, IconSettings } from "../icons";

type Props = {
  value: string;
  onChange: (value: string) => void;
  onClear: () => void;
  suggest: string[];
  suggestIndex: number;
  onPickSuggest: (name: string) => void;
  trailing?: ReactNode;
  paused?: boolean;
  compact?: boolean;
  onToggleMode?: () => void;
  onSettings?: () => void;
};

/** Header: wordmark, divider, borderless search, segmented toggle, mode button. */
export function SearchBar({
  value,
  onChange,
  onClear,
  suggest,
  suggestIndex,
  onPickSuggest,
  trailing,
  paused = false,
  compact = false,
  onToggleMode,
  onSettings,
}: Props) {
  const prefix = value.match(/(?:^|\s)#([^\s]*)$/)?.[1] ?? "";
  return (
    <div className="relative border-b border-[var(--line)]">
      <div
        className={`flex items-center px-4 ${
          compact ? "h-[52px] gap-3" : "h-[58px] gap-4"
        }`}
      >
        <span className={`wordmark ${paused ? "paused" : ""}`}>
          {compact ? (
            <>
              c<b>t</b>
            </>
          ) : (
            <>
              clip<b>trace</b>
            </>
          )}
        </span>
        {compact ? null : (
          <span
            className="h-5 w-px shrink-0"
            style={{ background: "var(--line)" }}
          />
        )}
        <label className="sr-only" htmlFor="q">
          Search clips
        </label>
        <input
          id="q"
          className="min-w-0 flex-1 placeholder:text-[13px] border-0 bg-transparent text-[16px] tracking-[-0.01em] outline-none ring-0 focus:outline-none focus-visible:outline-none placeholder:text-(--faint)"
          style={{ color: "var(--text)" }}
          placeholder="Search the trace…  #tag, last week"
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
            className="btn-ico !h-6 !w-6"
            aria-label="Clear search"
            onClick={onClear}
          >
            <IconClose className="h-3.5 w-3.5" />
          </button>
        ) : null}
        {compact ? null : trailing}
        <div className="space-x-2">
          <button
            type="button"
            className="btn-ico shrink-0"
            aria-label="Settings"
            title="Settings  Ctrl ,"
            onClick={onSettings}
          >
            <IconSettings className="h-[17px] w-[17px]" />
          </button>
          <button
            type="button"
            className="btn-ico shrink-0"
            aria-label={
              compact ? "Expand to full view" : "Collapse to quick paste"
            }
            title={compact ? "Expand (Tab)" : "Quick paste (Tab)"}
            onClick={onToggleMode}
          >
            {compact ? (
              <IconExpand className="h-[17px] w-[17px]" />
            ) : (
              <IconCollapse className="h-[17px] w-[17px]" />
            )}
          </button>
        </div>
      </div>
      {suggest.length > 0 ? (
        <ul
          id="suggest"
          role="listbox"
          className={`absolute left-4 right-4 z-20 overflow-hidden rounded-(--r-md) border-t-2 ${
            compact ? "top-[52px]" : "top-[58px]"
          }`}
          style={{
            background: "var(--panel)",
            borderColor: "var(--accent)",
            boxShadow: "var(--shadow-pop)",
          }}
        >
          {suggest.map((name, i) => (
            <li
              key={name}
              role="option"
              aria-selected={i === suggestIndex}
              className="flex h-9 cursor-pointer items-center px-3 font-mono text-[12px]"
              style={{
                background: i === suggestIndex ? "var(--raise)" : undefined,
                color: "var(--muted)",
              }}
              onMouseDown={(e) => {
                e.preventDefault();
                onPickSuggest(name);
              }}
            >
              #
              <span style={{ color: "var(--accent)" }}>
                {name.slice(0, prefix.length)}
              </span>
              {name.slice(prefix.length)}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
