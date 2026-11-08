import { useEffect, useMemo } from "react";
import type { ClipCard } from "../api";
import { dayHeading, dayLabel } from "../lib/time";
import { ClipRow } from "./ClipRow";
import { CompactRow } from "./CompactRow";
import { DayHeader } from "./DayHeader";

type Props = {
  cards: ClipCard[];
  selected: number;
  pickIds: number[];
  onSelect: (index: number) => void;
  onTogglePick: (id: number) => void;
  onMore: () => void;
  compact?: boolean;
};

type Row =
  | { type: "day"; day: string; index: number }
  | { type: "clip"; card: ClipCard; index: number };

export function ClipList({
  cards,
  selected,
  pickIds,
  onSelect,
  onTogglePick,
  onMore,
  compact = false,
}: Props) {
  const pickMode = pickIds.length > 0;
  const rows = useMemo(() => {
    const out: Row[] = [];
    let last = "";
    let firstRecent = true;
    cards.forEach((card, index) => {
      // Quick paste: pinned clips form their own section above recents.
      const section = compact && card.pinned;
      const key = section ? "pinned" : dayLabel(card.last_seen_at);
      if (key !== last) {
        let day = dayHeading(card.last_seen_at);
        if (section) day = "pinned";
        else if (compact && firstRecent) {
          day = `recent · ${dayLabel(card.last_seen_at).toLowerCase()}`;
        }
        if (!section) firstRecent = false;
        out.push({ type: "day", day, index });
        last = key;
      }
      out.push({ type: "clip", card, index });
    });
    return out;
  }, [cards, compact]);

  useEffect(() => {
    const id = cards[selected]?.id;
    if (id == null) return;
    document.getElementById(`clip-${id}`)?.scrollIntoView({ block: "nearest" });
  }, [cards, selected]);

  return (
    <ul
      className="h-full overflow-x-hidden overflow-y-auto px-1"
      role="listbox"
      aria-label="Clips"
      onScroll={(e) => {
        const el = e.currentTarget;
        if (el.scrollTop + el.clientHeight > el.scrollHeight - 40) onMore();
      }}
    >
      {rows.map((row) =>
        row.type === "day" ? (
          <li key={`d-${row.day}-${row.index}`} role="presentation">
            <DayHeader label={row.day} lit={row.index <= selected} compact={compact} />
          </li>
        ) : (
          <li key={row.card.id}>
            {compact ? (
              <CompactRow
                card={row.card}
                number={row.index + 1}
                selected={row.index === selected}
                past={row.index < selected}
                picked={pickIds.includes(row.card.id)}
                pickMode={pickMode}
                onSelect={() => onSelect(row.index)}
                onTogglePick={() => onTogglePick(row.card.id)}
              />
            ) : (
            <ClipRow
              card={row.card}
              selected={row.index === selected}
              past={row.index < selected}
              picked={pickIds.includes(row.card.id)}
              pickMode={pickMode}
              onSelect={() => onSelect(row.index)}
              onTogglePick={() => onTogglePick(row.card.id)}
            />
            )}
          </li>
        ),
      )}
    </ul>
  );
}
