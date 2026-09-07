import { describe, expect, it } from "vitest";

import { formatBytes, formatDate, formatDuration, formatPercent } from "./format";

describe("formatBytes", () => {
  it("names an absent measurement instead of showing a confident zero", () => {
    // The reason this matters: a "0 B" that really means "not measured yet" is
    // a lie the user cannot see through.
    expect(formatBytes(null)).toBe("unknown");
    expect(formatBytes(undefined)).toBe("unknown");
    expect(formatBytes(NaN)).toBe("unknown");
    expect(formatBytes(Infinity)).toBe("unknown");
  });

  it("reports a real zero as zero", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("switches from bytes to KiB at exactly 1024", () => {
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1024)).toBe("1.0 KiB");
  });

  it("drops the decimal at 100 and keeps it below", () => {
    // One decimal below 100, none at or above, so a live readout does not churn
    // digits while staying precise enough to watch a value move.
    expect(formatBytes(102297)).toBe("99.9 KiB"); // 99.899 KiB
    expect(formatBytes(102400)).toBe("100 KiB"); // exactly 100 KiB
  });

  it("climbs through the binary units", () => {
    expect(formatBytes(1024 ** 2)).toBe("1.0 MiB");
    expect(formatBytes(1024 ** 3)).toBe("1.0 GiB");
    expect(formatBytes(1024 ** 4)).toBe("1.0 TiB");
  });

  it("stops at TiB rather than inventing a larger unit", () => {
    expect(formatBytes(1024 ** 5)).toBe("1024 TiB");
  });
});

describe("formatPercent", () => {
  it("says sampling when there is no rate yet", () => {
    // One CPU sample cannot yield a rate, so the first poll has nothing to show.
    expect(formatPercent(null)).toBe("sampling");
    expect(formatPercent(undefined)).toBe("sampling");
    expect(formatPercent(NaN)).toBe("sampling");
  });

  it("keeps one decimal", () => {
    expect(formatPercent(0)).toBe("0.0 %");
    expect(formatPercent(12.34)).toBe("12.3 %");
    expect(formatPercent(100)).toBe("100.0 %");
  });
});

describe("formatDuration", () => {
  it("shows seconds under a minute", () => {
    expect(formatDuration(0)).toBe("0s");
    expect(formatDuration(999)).toBe("0s");
    expect(formatDuration(59_000)).toBe("59s");
  });

  it("shows minutes and seconds under an hour", () => {
    expect(formatDuration(60_000)).toBe("1m 0s");
    expect(formatDuration(65_000)).toBe("1m 5s");
    expect(formatDuration(3_599_000)).toBe("59m 59s");
  });

  it("drops seconds once there are hours", () => {
    expect(formatDuration(3_600_000)).toBe("1h 0m");
    expect(formatDuration(3_660_000)).toBe("1h 1m");
  });
});

describe("formatDate", () => {
  it("says never played rather than showing the epoch", () => {
    // The backend sends null for a game with no recorded play time. Rendering
    // that as 1 January 1970 would look like a real date.
    expect(formatDate(null)).toBe("never played");
  });

  it("renders a real timestamp as a local date", () => {
    // The exact string is locale dependent, so assert the parts rather than a
    // formatting that varies by machine.
    const formatted = formatDate(1_700_000_000);
    expect(formatted).not.toBe("never played");
    expect(formatted).toContain("2023");
  });
});
