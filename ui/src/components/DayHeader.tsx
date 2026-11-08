export function DayHeader({ label }: { label: string }) {
  return (
    <p className="font-label-xs bg-surface-container-lowest px-3 py-1.5 text-on-surface-variant">
      {label}
    </p>
  );
}
