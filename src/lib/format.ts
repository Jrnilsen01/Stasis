/**
 * Formatters for measured values.
 *
 * Every function here takes a real number from the backend. None of them
 * invents a value: an absent measurement is named in words ("unknown",
 * "never") rather than shown as a dash or a confident zero, because a zero
 * that means "not measured yet" is a lie the user cannot see through.
 */

/** Binary units, because this is memory and the OS reports it that way. */
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes)) {
    return "unknown";
  }
  if (bytes < 1024) return `${bytes} B`;

  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes / 1024;
  let unit = 0;

  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }

  // One decimal below 100, none above: enough precision to watch a value move
  // without the digits churning.
  return `${value < 100 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

export function formatPercent(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "sampling";
  }
  return `${value.toFixed(1)} %`;
}

export function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${seconds}s`;
  return `${seconds}s`;
}

/** Unix seconds to a plain local date. Absent means never played, not "today". */
export function formatDate(unixSeconds: number | null): string {
  if (unixSeconds === null) return "never played";
  return new Date(unixSeconds * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}
