import { IconPin } from "../icons";

type Tab = "recent" | "pinned";

type Props = {
  tab: Tab;
  total: number;
  pinned: number;
  onChange: (tab: Tab) => void;
};

export function Tabs({ tab, total, pinned, onChange }: Props) {
  const item = (id: Tab, label: string, count: number) => {
    const on = tab === id;
    return (
      <button
        type="button"
        role="tab"
        aria-selected={on}
        className={`font-label-sm inline-flex cursor-pointer items-center gap-1 rounded-full px-2 py-0.5 ${
          on
            ? "bg-surface-container-lowest text-primary"
            : "text-on-surface-variant hover:text-on-surface"
        }`}
        onClick={() => onChange(id)}
      >
        {id === "pinned" ? <IconPin className="h-3 w-3" /> : null}
        {label}
        <span className="tabular-nums opacity-70">{count}</span>
      </button>
    );
  };
  return (
    <div
      className="flex shrink-0 items-center gap-px"
      role="tablist"
    >
      {item("recent", "All", total)}
      {item("pinned", "Pinned", pinned)}
    </div>
  );
}
