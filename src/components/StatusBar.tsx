import type { Footprint } from "../lib/api";
import { formatBytes, formatDuration, formatPercent } from "../lib/format";

type StatusBarProps = {
  footprint: Footprint | null;
  error: string | null;
};

/**
 * The permanent readout.
 *
 * This bar is the product's argument made visible: the whole pitch is that the
 * overlay host is light, so its own cost is on screen at all times rather than
 * buried in an about box. Every number here is measured from this process on
 * each poll. Nothing is a placeholder, and a value that has not been measured
 * yet says so in words instead of showing zero.
 *
 * Memory carries the accent because it is the single number the product is
 * judged on. CPU and uptime sit in neutral text so the emphasis means
 * something.
 */
export function StatusBar({ footprint, error }: StatusBarProps) {
  return (
    <footer className="statusbar">
      <div className="metric">
        <span className="metric__label">MEMORY</span>
        <span
          className={
            footprint
              ? "metric__value metric__value--accent"
              : "metric__value metric__value--unknown"
          }
        >
          {footprint ? formatBytes(footprint.memoryBytes) : "reading"}
        </span>
      </div>

      <div className="metric">
        <span className="metric__label">CPU</span>
        <span
          className={
            footprint?.cpuPercent === null || footprint === null
              ? "metric__value metric__value--unknown"
              : "metric__value"
          }
        >
          {footprint ? formatPercent(footprint.cpuPercent) : "reading"}
        </span>
      </div>

      <div className="metric">
        <span className="metric__label">UPTIME</span>
        <span className="metric__value">
          {footprint ? formatDuration(footprint.uptimeMs) : "reading"}
        </span>
      </div>

      <div className="statusbar__spacer" />

      {/* Status is a word first. The coloured square is the second signal, so
          the state never depends on distinguishing hue. */}
      {error ? (
        <span className="pill">
          <span className="pill__mark pill__mark--error" />
          Sampler stopped: {error}
        </span>
      ) : (
        <span className="pill">
          <span className="pill__mark pill__mark--ok" />
          {footprint
            ? `Sampling this process, pid ${footprint.pid}`
            : "Starting the sampler"}
        </span>
      )}
    </footer>
  );
}
