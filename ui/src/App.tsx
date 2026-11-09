import { useState } from "react";
import { tauriAvailable } from "./api";
import { ClipBrowser } from "./components/ClipBrowser";
import { KeyHints } from "./components/KeyHints";
import { KeywordNote } from "./components/KeywordNote";
import { Overlays } from "./components/Overlays";
import { PopupFrame } from "./components/PopupFrame";
import { SearchHeader } from "./components/SearchHeader";
import { ToastStack } from "./components/ToastStack";
import { useApi } from "./hooks/useApi";
import { useClipActions } from "./hooks/useClipActions";
import { useClipSearch } from "./hooks/useClipSearch";
import { useClips } from "./hooks/useClips";
import { usePopupLifecycle } from "./hooks/usePopupLifecycle";
import { useShortcuts } from "./hooks/useShortcuts";
import { useTheme } from "./hooks/useTheme";
import { useToasts } from "./hooks/useToasts";
import { useWelcome } from "./hooks/useWelcome";

const focusSearch = () =>
  window.setTimeout(() => document.getElementById("q")?.focus(), 0);

export default function App() {
  const api = useApi();
  // Quick paste (compact) is the narrow popup; full view adds the preview pane.
  const [compact, setCompact] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const theme = useTheme();
  const toasts = useToasts();
  const clips = useClips(api, compact);
  const search = useClipSearch(clips);
  const actions = useClipActions({ api, clips, toasts });
  const welcome = useWelcome(api, toasts);

  function toggleMode() {
    const next = !compact;
    clips.anchorSelection();
    setCompact(next);
    void api?.setMode(next);
    focusSearch();
  }

  /** A click copies the clip. Quick paste closes afterwards, like Enter; full view
   *  stays open so the preview can be read. While clips are checked for a bulk
   *  action, a click only moves the cursor. */
  function selectRow(index: number) {
    clips.selectRow(index);
    const id = clips.cards[index]?.id;
    if (!clips.pickIds.length && id != null) void actions.copy(id, compact);
  }

  usePopupLifecycle((cache) => {
    setSettingsOpen(false);
    clips.resetForOpen(cache);
  }, actions.flushDeletes);

  useShortcuts({
    clips,
    search,
    actions,
    welcome,
    settingsOpen,
    setSettingsOpen,
    toggleMode,
    toggleTheme: theme.toggle,
  });

  const frame = (
    <PopupFrame
      compact={compact}
      onDragStart={() => void api?.startDrag()}
    >
      {clips.note?.startsWith("keyword-only") ? <KeywordNote /> : null}
      <SearchHeader
        query={clips.query}
        tab={clips.tab}
        total={clips.total}
        pinned={clips.pinned}
        pickCount={clips.pickIds.length}
        visibleCount={clips.cards.length}
        paused={clips.paused}
        compact={compact}
        suggest={search.suggest}
        suggestIndex={search.suggestIndex}
        onSearch={search.search}
        onPickSuggestion={search.pickSuggestion}
        onChangeTab={clips.changeTab}
        onToggleMode={toggleMode}
        onOpenSettings={() => setSettingsOpen(true)}
        onSelectAll={clips.selectAll}
        onClearPicks={clips.clearPicks}
        onDeletePicked={actions.deletePicked}
      />
      <div className="relative mt-space-xs min-h-0 flex-1 overflow-hidden">
        <ClipBrowser
          cards={clips.cards}
          current={clips.current}
          selected={clips.selected}
          pickIds={clips.pickIds}
          allTags={clips.allTags}
          query={clips.query}
          total={clips.total}
          emptyKind={clips.emptyKind}
          paused={clips.paused}
          compact={compact}
          copied={toasts.copied}
          onSelectRow={selectRow}
          onTogglePick={clips.togglePick}
          onMore={clips.loadMore}
          onClearSearch={search.clearSearch}
          onBrowseRecent={() => clips.changeTab("recent")}
          onCopy={() => void actions.copy()}
          onPin={() => void actions.togglePin()}
          onSecret={actions.askSecret}
          onDelete={actions.askDelete}
          onAddTag={(tag) => void actions.addTag(tag)}
          onRemoveTag={(tag) => void actions.removeTag(tag)}
        />
      </div>
      <ToastStack
        undoId={actions.undoId}
        copied={toasts.copied}
        error={toasts.error}
        onUndo={actions.undoDelete}
        onDismissUndo={actions.dismissUndo}
        onDismissError={toasts.clearError}
      />
      <KeyHints
        empty={clips.cards.length === 0}
        compact={compact}
        theme={theme.shown}
        onToggleTheme={theme.toggle}
      />
      <Overlays
        api={api}
        compact={compact}
        settings={{
          open: settingsOpen,
          theme: theme.theme,
          onTheme: theme.setTheme,
          onClose: () => {
            setSettingsOpen(false);
            focusSearch();
          },
          onError: (message) => toasts.showError(message),
          onShowWelcome: () => {
            setSettingsOpen(false);
            welcome.show();
          },
        }}
        welcome={{
          open: welcome.open,
          hotkey: welcome.hotkey,
          onDone: welcome.finish,
        }}
        secret={{
          open: actions.secretPrompt,
          specimen: clips.current?.body ?? "",
          onCancel: actions.cancelSecret,
          onConfirm: () => void actions.confirmSecret(),
        }}
      />
    </PopupFrame>
  );

  if (tauriAvailable()) return <div className="h-full">{frame}</div>;
  return (
    <div className="flex min-h-full items-center justify-center bg-(--panel) p-8">
      {frame}
    </div>
  );
}
