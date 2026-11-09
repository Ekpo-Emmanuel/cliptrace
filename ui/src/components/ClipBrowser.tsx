import type { ClipCard, TagCount } from "../api";
import type { EmptyKind } from "../hooks/useClips";
import { ClipList } from "./ClipList";
import { EmptyState } from "./EmptyState";
import { PausedBand } from "./PausedBand";
import { PreviewPane } from "./PreviewPane";

type Props = {
  cards: ClipCard[];
  /** The selected clip with its full text; undefined when the list is empty. */
  current: ClipCard | undefined;
  selected: number;
  pickIds: number[];
  allTags: TagCount[];
  query: string;
  total: number;
  emptyKind: EmptyKind;
  paused: boolean;
  compact: boolean;
  copied: boolean;
  onSelectRow: (index: number) => void;
  onTogglePick: (id: number) => void;
  onMore: () => void;
  onClearSearch: () => void;
  onBrowseRecent: () => void;
  onCopy: () => void;
  onPin: () => void;
  onSecret: () => void;
  onDelete: () => void;
  onAddTag: (tag: string) => void;
  onRemoveTag: (tag: string) => void;
};

/** The popup body: the clip list with its preview, or an empty state. */
export function ClipBrowser({
  cards,
  current,
  selected,
  pickIds,
  allTags,
  query,
  total,
  emptyKind,
  paused,
  compact,
  copied,
  onSelectRow,
  onTogglePick,
  onMore,
  onClearSearch,
  onBrowseRecent,
  onCopy,
  onPin,
  onSecret,
  onDelete,
  onAddTag,
  onRemoveTag,
}: Props) {
  if (cards.length === 0) {
    return (
      <EmptyState
        kind={emptyKind}
        query={query}
        total={total}
        compact={compact}
        onClear={onClearSearch}
        onBrowse={onBrowseRecent}
      />
    );
  }
  return (
    <div
      className={`grid h-full min-h-0 ${
        compact
          ? "grid-cols-[minmax(0,1fr)]"
          : "grid-cols-[minmax(0,344px)_minmax(0,1fr)]"
      }`}
    >
      <div className="flex min-h-0 flex-col">
        {paused ? <PausedBand /> : null}
        <div className="min-h-0 flex-1">
          <ClipList
            cards={cards}
            selected={selected}
            pickIds={pickIds}
            onSelect={onSelectRow}
            onTogglePick={onTogglePick}
            compact={compact}
            onMore={onMore}
          />
        </div>
      </div>
      {compact ? null : (
        <PreviewPane
          card={current}
          position={selected + 1}
          allTags={allTags}
          copied={copied}
          onCopy={onCopy}
          onPin={onPin}
          onSecret={onSecret}
          onDelete={onDelete}
          onAddTag={onAddTag}
          onRemoveTag={onRemoveTag}
        />
      )}
    </div>
  );
}
