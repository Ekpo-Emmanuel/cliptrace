import { PickBar } from "./PickBar";
import { SearchBar } from "./SearchBar";
import { Tabs } from "./Tabs";
import type { Tab } from "../hooks/useClips";

type Props = {
  query: string;
  tab: Tab;
  total: number;
  pinned: number;
  /** Clips currently checked; the selection bar replaces the search row when any. */
  pickCount: number;
  visibleCount: number;
  paused: boolean;
  compact: boolean;
  suggest: string[];
  suggestIndex: number;
  onSearch: (value: string) => void;
  onPickSuggestion: (name: string) => void;
  onChangeTab: (tab: Tab) => void;
  onToggleMode: () => void;
  onOpenSettings: () => void;
  onSelectAll: () => void;
  onClearPicks: () => void;
  onDeletePicked: () => void;
};

/** Top of the popup: the search box with its tabs, or the bulk-selection bar. */
export function SearchHeader({
  query,
  tab,
  total,
  pinned,
  pickCount,
  visibleCount,
  paused,
  compact,
  suggest,
  suggestIndex,
  onSearch,
  onPickSuggestion,
  onChangeTab,
  onToggleMode,
  onOpenSettings,
  onSelectAll,
  onClearPicks,
  onDeletePicked,
}: Props) {
  return (
    <div className="relative">
      {pickCount > 0 ? (
        <PickBar
          count={pickCount}
          total={visibleCount}
          onSelectAll={onSelectAll}
          onClear={onClearPicks}
          onDelete={onDeletePicked}
        />
      ) : null}
      <SearchBar
        paused={paused}
        value={query}
        onChange={onSearch}
        onClear={() => onSearch("")}
        suggest={suggest}
        suggestIndex={suggestIndex}
        onPickSuggest={onPickSuggestion}
        compact={compact}
        onToggleMode={onToggleMode}
        onSettings={onOpenSettings}
        trailing={
          <Tabs tab={tab} total={total} pinned={pinned} onChange={onChangeTab} />
        }
      />
    </div>
  );
}
