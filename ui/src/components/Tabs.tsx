type Tab = "recent" | "pinned";

type Props = {
  tab: Tab;
  total: number;
  pinned: number;
  onChange: (tab: Tab) => void;
};

export function Tabs({ tab, total, pinned, onChange }: Props) {
  const item = (id: Tab, label: string, count: number) => (
    <button
      type="button"
      role="tab"
      aria-selected={tab === id}
      className="seg-item"
      onClick={() => onChange(id)}
    >
      <span className={id === "pinned" ? "seg-diamond" : "seg-dot"} />
      {label}
      <span className="seg-count tabular-nums">{count}</span>
    </button>
  );
  return (
    <div className="seg shrink-0" role="tablist">
      {item("recent", "all", total)}
      {item("pinned", "pinned", pinned)}
    </div>
  );
}
