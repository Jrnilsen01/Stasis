/**
 * The Stasis mark: four viewfinder corner brackets framing a single copper
 * square.
 *
 * Drawn as inline SVG on the 1024 design grid used by `tools/make-icon.py`, so
 * the interface and the app icon are the same geometry rather than two drawings
 * that drift apart. The numbers below are that script's constants.
 *
 * The tile background is deliberately omitted. In the app the mark sits
 * directly on the titlebar surface; a dark tile here would read as a square
 * inside a square.
 */

const INSET = 172;
const ARM = 270;
const STROKE = 84;
const MARK = 216;

const GRID = 1024;
const FAR = GRID - INSET;
const HALF = MARK / 2;
const CENTRE = GRID / 2;

export function Mark({ size = 18 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${GRID} ${GRID}`}
      // Decorative: the product name sits next to it in real text, so
      // announcing this twice would only add noise for screen readers.
      aria-hidden="true"
      focusable="false"
      // Lets a drag on the mark fall through to the titlebar drag region.
      style={{ pointerEvents: "none", display: "block", flex: "none" }}
    >
      <g fill="var(--text-primary)">
        {/* top-left */}
        <rect x={INSET} y={INSET} width={ARM} height={STROKE} />
        <rect x={INSET} y={INSET} width={STROKE} height={ARM} />
        {/* top-right */}
        <rect x={FAR - ARM} y={INSET} width={ARM} height={STROKE} />
        <rect x={FAR - STROKE} y={INSET} width={STROKE} height={ARM} />
        {/* bottom-left */}
        <rect x={INSET} y={FAR - STROKE} width={ARM} height={STROKE} />
        <rect x={INSET} y={FAR - ARM} width={STROKE} height={ARM} />
        {/* bottom-right */}
        <rect x={FAR - ARM} y={FAR - STROKE} width={ARM} height={STROKE} />
        <rect x={FAR - STROKE} y={FAR - ARM} width={STROKE} height={ARM} />
      </g>

      {/* The observed mark, in the one accent colour. Same glyph as the status
          square in the footer, which is what ties the icon to the interface. */}
      <rect
        x={CENTRE - HALF}
        y={CENTRE - HALF}
        width={MARK}
        height={MARK}
        fill="var(--copper-fill)"
      />
    </svg>
  );
}
