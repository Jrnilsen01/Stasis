export type ViewId = "library" | "games" | "settings";

type RailProps = {
  active: ViewId;
  onSelect: (view: ViewId) => void;
  /** Real counts, or null while the view has not loaded its data yet. */
  moduleCount: number | null;
  gameCount: number | null;
};

/**
 * Three destinations, all of which exist. There is no "Store", "Account", or
 * "Community" here: those would be links to nothing, and a rail that promises
 * screens the app does not have is the fastest way to lose someone's trust.
 *
 * The labels are text rather than icons. An icon rail would need a whole icon
 * language to be legible, and three words are clearer than three glyphs.
 */
export function Rail({ active, onSelect, moduleCount, gameCount }: RailProps) {
  const items: Array<{ id: ViewId; label: string; count: number | null }> = [
    { id: "library", label: "Modules", count: moduleCount },
    { id: "games", label: "Games", count: gameCount },
    { id: "settings", label: "Settings", count: null },
  ];

  return (
    <nav className="rail" aria-label="Sections">
      <div className="rail__heading">SECTIONS</div>
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          className="rail__item"
          // aria-current is both the accessible state and the styling hook, so
          // the two can never drift apart.
          aria-current={active === item.id ? "page" : undefined}
          onClick={() => onSelect(item.id)}
        >
          <span>{item.label}</span>
          {item.count !== null && (
            <span className="rail__count">{item.count}</span>
          )}
        </button>
      ))}
    </nav>
  );
}
