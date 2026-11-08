import { IconMoon, IconSun } from "../icons";

type Props = {
  empty?: boolean;
  compact?: boolean;
  /** Theme on screen now. The toggle is shown in full view only. */
  theme?: "light" | "dark";
  onToggleTheme?: () => void;
};

/** Three hints at most. Everything else lives in button tooltips. */
export function KeyHints({
  empty = false,
  compact = false,
  theme = "dark",
  onToggleTheme,
}: Props) {
  const Item = ({ k, v }: { k: string; v: string }) => (
    <span className="flex items-center gap-1.5">
      <kbd className="keycap">{k}</kbd>
      <span>{v}</span>
    </span>
  );
  const next = theme === "dark" ? "light" : "dark";
  return (
    <footer
      className={`relative flex shrink-0 items-center gap-5 border-t px-4 font-mono text-[10px] ${
        compact ? "h-[46px]" : "h-[40px]"
      }`}
      style={{ borderColor: "var(--line)", color: "var(--faint)" }}
    >
      {compact ? (
        <>
          <Item k="↵" v="paste" />
          <Item k="esc" v="close" />
        </>
      ) : empty ? (
        <Item k="esc" v="close" />
      ) : (
        <>
          <Item k="↑↓" v="move" />
          <Item k="↵" v="copy" />
          <Item k="esc" v="close" />
        </>
      )}
      {compact ? null : (
        <button
          type="button"
          className="btn-ico ml-auto !h-7 !w-7"
          aria-label={`Switch to ${next} theme`}
          title={`Switch to ${next} theme`}
          onClick={onToggleTheme}
        >
          {theme === "dark" ? (
            <IconSun className="h-[15px] w-[15px]" />
          ) : (
            <IconMoon className="h-[15px] w-[15px]" />
          )}
        </button>
      )}
    </footer>
  );
}
