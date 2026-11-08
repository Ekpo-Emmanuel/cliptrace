type Props = {
  empty?: boolean;
};

export function KeyHints({ empty = false }: Props) {
  const Item = ({ k, v }: { k: string; v: string }) => (
    <span className="flex items-center gap-1">
      <kbd className="font-label-sm kbd-bevel rounded-sm bg-surface-container-high px-1.5 py-0.5 text-on-surface">
        {k}
      </kbd>
      <span>{v}</span>
    </span>
  );
  return (
    <footer className="font-label-sm relative flex shrink-0 flex-wrap gap-x-3 gap-y-1 border-t border-outline-variant/30 bg-white px-space-md py-space-xs text-on-surface-variant">
      {empty ? (
        <Item k="Esc" v="close" />
      ) : (
        <>
          <Item k="↑↓" v="navigate" />
          <Item k="Enter" v="copy" />
          <Item k="Ctrl+P" v="pin" />
          <Item k="Ctrl+T" v="tag" />
          <Item k="Ctrl+L" v="mark secret" />
          <Item k="Del" v="delete" />
          <Item k="Esc" v="close" />
        </>
      )}
    </footer>
  );
}
