export function DayHeader({
  label,
  lit,
  compact = false,
}: {
  label: string;
  lit: boolean;
  compact?: boolean;
}) {
  return (
    <div className={`tday ${compact ? "q" : ""}`}>
      <span />
      <span className="tspine h-full">
        {lit ? <span className="tlit bottom-0" /> : null}
      </span>
      <span>{label}</span>
    </div>
  );
}
