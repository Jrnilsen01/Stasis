import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { ScanResult } from "../lib/api";

// The views reach Rust through the Tauri bridge, which is absent in a test
// process. Mocking the api module is the boundary: everything below it is
// covered by the Rust tests under src-tauri.
vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return { ...actual, scanSteamGames: vi.fn() };
});

const { scanSteamGames } = await import("../lib/api");
const { GamesView } = await import("./GamesView");

const scan = vi.mocked(scanSteamGames);

function renderWith(result: ScanResult) {
  scan.mockResolvedValue(result);
  return render(
    <GamesView steamPathOverride="" onCountChange={() => {}} onGoToSettings={() => {}} />,
  );
}

beforeEach(() => {
  scan.mockReset();
});

describe("Steam found but unreadable", () => {
  // Implemented but never triggered before this test: reaching it in the real
  // app needs a Steam folder that exists and denies a directory listing.
  const unreadable: ScanResult = {
    status: "unreadable",
    path: "D:\\Steam",
    reason: "Access is denied. (os error 5)",
  };

  it("distinguishes a read failure from an empty library", async () => {
    renderWith(unreadable);

    expect(await screen.findByText("Steam was found but could not be read")).toBeVisible();
    expect(screen.getByText("UNREADABLE")).toBeVisible();
  });

  it("shows the path and the operating system's own reason", async () => {
    renderWith(unreadable);

    const body = await screen.findByText(/Access is denied/);
    expect(body).toHaveTextContent("D:\\Steam");
    expect(body).toHaveTextContent("os error 5");
  });

  it("offers a retry and a way to point somewhere else", async () => {
    renderWith(unreadable);

    expect(await screen.findByRole("button", { name: "Try reading again" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Point at a different folder" })).toBeEnabled();
  });

  it("calls back with a null count rather than zero", async () => {
    // Zero would render as "0 games" in the rail, which claims a successful
    // scan that found nothing. The count is unknown here, not zero.
    const onCountChange = vi.fn();
    scan.mockResolvedValue(unreadable);
    render(
      <GamesView steamPathOverride="" onCountChange={onCountChange} onGoToSettings={() => {}} />,
    );

    await screen.findByText("Steam was found but could not be read");
    expect(onCountChange).toHaveBeenCalledWith(null);
    expect(onCountChange).not.toHaveBeenCalledWith(0);
  });
});

describe("Steam found with zero games", () => {
  const empty: ScanResult = {
    status: "scanned",
    steamRoot: "C:\\Program Files (x86)\\Steam",
    libraries: ["C:\\Program Files (x86)\\Steam"],
    games: [],
  };

  it("says the library is empty rather than that Steam is missing", async () => {
    renderWith(empty);

    expect(await screen.findByText("Steam is installed, with nothing in it")).toBeVisible();
    expect(screen.getByText("NO GAMES")).toBeVisible();
  });

  it("reports what was actually read", async () => {
    renderWith(empty);

    const body = await screen.findByText(/library folder/);
    expect(body).toHaveTextContent("C:\\Program Files (x86)\\Steam");
    expect(body).toHaveTextContent("1 library folder");
  });

  it("pluralises the folder count", async () => {
    renderWith({ ...empty, libraries: ["C:\\Steam", "D:\\SteamLibrary"] });

    expect(await screen.findByText(/2 library folders/)).toBeVisible();
  });

  it("reports a count of zero, because zero is the true answer here", async () => {
    // The opposite of the unreadable case: the scan succeeded, so zero is a
    // measurement rather than an absence.
    const onCountChange = vi.fn();
    scan.mockResolvedValue(empty);
    render(
      <GamesView steamPathOverride="" onCountChange={onCountChange} onGoToSettings={() => {}} />,
    );

    await screen.findByText("Steam is installed, with nothing in it");
    expect(onCountChange).toHaveBeenCalledWith(0);
  });
});
