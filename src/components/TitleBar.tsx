import { getCurrentWindow } from "@tauri-apps/api/window";

import { PRODUCT_NAME } from "../lib/api";
import { Mark } from "./Mark";

/**
 * The window is frameless (decorations are off in tauri.conf.json) because the
 * machined titlebar is part of the hardware identity rather than a decoration
 * layered on top of the OS chrome. That makes these three controls real
 * responsibilities: without them the window cannot be minimised or closed.
 */

const glyphProps = {
  width: 10,
  height: 10,
  viewBox: "0 0 10 10",
  "aria-hidden": true,
  focusable: false,
  stroke: "currentColor",
  strokeWidth: 1.2,
  fill: "none",
} as const;

export function TitleBar() {
  const appWindow = getCurrentWindow();

  return (
    <header className="titlebar">
      {/* Only this region drags, so the buttons stay clickable. */}
      <div className="titlebar__drag" data-tauri-drag-region>
        <Mark size={18} />
        <span className="wordmark" data-tauri-drag-region>
          {PRODUCT_NAME}
        </span>
        <span className="wordmark__tag" data-tauri-drag-region>
          overlay host
        </span>
      </div>

      <div className="window-controls">
        <button
          type="button"
          aria-label="Minimise window"
          onClick={() => void appWindow.minimize()}
        >
          <svg {...glyphProps}>
            <line x1="1" y1="5" x2="9" y2="5" />
          </svg>
        </button>
        <button
          type="button"
          aria-label="Maximise or restore window"
          onClick={() => void appWindow.toggleMaximize()}
        >
          <svg {...glyphProps}>
            <rect x="1.5" y="1.5" width="7" height="7" />
          </svg>
        </button>
        <button
          type="button"
          data-action="close"
          aria-label="Close window"
          onClick={() => void appWindow.close()}
        >
          <svg {...glyphProps}>
            <line x1="1.5" y1="1.5" x2="8.5" y2="8.5" />
            <line x1="8.5" y1="1.5" x2="1.5" y2="8.5" />
          </svg>
        </button>
      </div>
    </header>
  );
}
