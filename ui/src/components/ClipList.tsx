import { useEffect, useMemo } from "react";
import type { ClipCard } from "../api";
import { dayLabel } from "../lib/time";
import { ClipRow } from "./ClipRow";
import { DayHeader } from "./DayHeader";

type Props = {
  cards: ClipCard[];
  selected: number;
  pickIds: number[];
  onSelect: (index: number) => void;
  onTogglePick: (id: number) => void;
  onPin: (card: ClipCard) => void;
  onTag: (index: number) => void;
  onMore: () => void;
};

type Row =
  | { type: "day"; day: string }
  | { type: "clip"; card: ClipCard; index: number };

export function ClipList({
  cards,
  selected,
  pickIds,
  onSelect,
  onTogglePick,
  onPin,
  onTag,
  onMore,
}: Props) {
  const pickMode = pickIds.length > 0;
  const rows = useMemo(() => {
    const out: Row[] = [];
    let last = "";
    cards.forEach((card, index) => {
      const day = dayLabel(card.last_seen_at);
      if (day !== last) {
        out.push({ type: "day", day });
        last = day;
      }
      out.push({ type: "clip", card, index });
    });
    return out;
  }, [cards]);

  useEffect(() => {
    const id = cards[selected]?.id;
    if (id == null) return;
    document.getElementById(`clip-${id}`)?.scrollIntoView({ block: "nearest" });
  }, [cards, selected]);

  return (
    <ul
      className="h-full divide-y divide-outline-variant/20 overflow-x-hidden overflow-y-auto"
      role="listbox"
      aria-label="Clips"
      onScroll={(e) => {
        const el = e.currentTarget;
        if (el.scrollTop + el.clientHeight > el.scrollHeight - 40) onMore();
      }}
    >
      {rows.map((row) =>
        row.type === "day" ? (
          <li key={`d-${row.day}`} role="presentation" className="border-none">
            <DayHeader label={row.day} />
          </li>
        ) : (
          <li key={row.card.id}>
            <ClipRow
              card={row.card}
              selected={row.index === selected}
              picked={pickIds.includes(row.card.id)}
              pickMode={pickMode}
              onSelect={() => onSelect(row.index)}
              onTogglePick={() => onTogglePick(row.card.id)}
              onPin={() => onPin(row.card)}
              onTag={() => onTag(row.index)}
            />
          </li>
        ),
      )}
    </ul>
  );
}
