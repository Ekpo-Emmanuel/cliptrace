import type { ClipCard } from "../api";

type Props = {
  card: ClipCard;
  onCopy: () => void;
  onPin: () => void;
  onSecret: () => void;
  onDelete: () => void;
};

export function ActionsBar({ card, onCopy, onPin, onSecret, onDelete }: Props) {
  const btn = "font-label-md rounded-md px-space-sm py-space-xs";
  return (
    <div className="flex flex-wrap gap-space-xs">
      <button type="button" className={`${btn} bg-accent text-on-primary`} onClick={onCopy}>
        Copy
      </button>
      <button
        type="button"
        className={`${btn} border border-outline-variant/40 bg-transparent text-on-surface`}
        onClick={onPin}
      >
        {card.pinned ? "Unpin" : "Pin"}
      </button>
      {card.kind !== "secret" ? (
        <button
          type="button"
          className={`${btn} border border-outline-variant/40 bg-transparent text-on-surface`}
          onClick={onSecret}
        >
          Mark as secret
        </button>
      ) : null}
      <button
        type="button"
        className={`${btn} border border-outline-variant/40 bg-transparent text-error`}
        onClick={onDelete}
      >
        Delete
      </button>
    </div>
  );
}
