import type { ReactNode } from "react";

type Props = {
  /** Quick paste (narrow) or full view. */
  compact: boolean;
  onDragStart: () => void;
  children: ReactNode;
};

/** The popup window shell: size by mode, rounded frame and the drag handle. */
export function PopupFrame({ compact, onDragStart, children }: Props) {
  return (
    <div
      className={`relative flex flex-col overflow-hidden rounded-(--r-window) bg-(--bg) text-(--text) border border-[var(--line)] ${
        compact ? "tq h-[440px] w-[380px]" : "tr h-[580px] w-[880px]"
      }`}
    >
      <div
        className="flex h-3 shrink-0 cursor-grab items-center justify-center active:cursor-grabbing"
        aria-label="Move window"
        onMouseDown={(e) => {
          if (e.button !== 0) return;
          e.preventDefault();
          onDragStart();
        }}
      >
        <span className="pointer-events-none h-0.5 w-8 rounded-full bg-(--line)" />
      </div>
      {children}
    </div>
  );
}
