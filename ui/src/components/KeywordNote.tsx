/** Shown when semantic search is unavailable and only keywords work. */
export function KeywordNote() {
  return (
    <div
      className="px-4 py-1 font-mono text-[10px]"
      style={{ color: "var(--faint)" }}
    >
      keyword-only search · embeddings unavailable
    </div>
  );
}
