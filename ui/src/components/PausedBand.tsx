/** Reminder above the list while capture is paused. */
export function PausedBand() {
  return (
    <div className="paused-band" role="status">
      <span aria-hidden>❚❚</span>
      <span>
        <b style={{ color: "var(--text)" }}>Capture paused</b>
        {" · copies aren’t saved"}
      </span>
    </div>
  );
}
