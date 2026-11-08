import { useEffect, useState } from "react";
import { getApi, tauriAvailable, type BrowsePayload, type ClipCard } from "./api";

export default function App() {
  const inApp = tauriAvailable();
  const [payload, setPayload] = useState<BrowsePayload | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    async function load(cache?: { cards?: ClipCard[]; paused?: boolean }) {
      try {
        const api = await getApi();
        if (cache?.cards?.length && inApp) {
          if (!alive) return;
          setPayload({
            cards: cache.cards,
            pinned: cache.cards.filter((c) => c.pinned).length,
            note: null,
            paused: !!cache.paused,
          });
        }
        const next = await api.browseClips({
          query: "",
          tab: "recent",
          tag: "",
          offset: 0,
        });
        if (!alive) return;
        setPayload(next);
      } catch (err) {
        if (!alive) return;
        setError(err instanceof Error ? err.message : String(err));
      }
    }
    window.cliptraceOnOpen = (cache) => {
      void load(cache);
    };
    void load();
    return () => {
      alive = false;
    };
  }, [inApp]);

  const frame = (
    <div className="flex h-[560px] w-[760px] flex-col overflow-hidden rounded-2xl border border-zinc-200 bg-zinc-100 text-zinc-900 shadow-xl dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-100">
      <header className="border-b border-zinc-200 px-4 py-3 text-sm font-semibold dark:border-zinc-700">
        Cliptrace
        {payload?.paused ? (
          <span className="ml-2 rounded-full bg-accent px-2 py-0.5 text-xs font-medium text-white">
            Capture paused
          </span>
        ) : null}
      </header>
      {payload?.note ? (
        <p className="px-4 pt-2 text-xs text-accent">{payload.note}</p>
      ) : null}
      <ul className="flex-1 overflow-auto p-2">
        {error ? <li className="p-4 text-sm text-red-700">{error}</li> : null}
        {!error && payload && payload.cards.length === 0 ? (
          <li className="p-4 text-sm text-zinc-500">Copy something and it’ll show up here.</li>
        ) : null}
        {payload?.cards.map((card) => (
          <li
            key={card.id}
            className="mb-1 rounded-xl border border-zinc-200 bg-white px-3 py-2 dark:border-zinc-700 dark:bg-zinc-800"
          >
            <div className="font-mono text-xs text-zinc-500">{card.kind}</div>
            <div className="truncate text-sm">{card.preview}</div>
            <div className="mt-1 text-xs text-zinc-500">
              {card.source_exe}
              {card.pinned ? " · pinned" : ""}
              {card.tags.length ? ` · ${card.tags.join(", ")}` : ""}
            </div>
          </li>
        ))}
      </ul>
    </div>
  );

  if (inApp) {
    return <div className="h-full bg-transparent">{frame}</div>;
  }

  return (
    <div className="flex min-h-full items-center justify-center bg-zinc-300 p-8 dark:bg-zinc-950">
      {frame}
    </div>
  );
}
