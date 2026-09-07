import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { Settings, SteamPathCheck } from "../lib/api";

// Same boundary as the other view tests: the disk answers below `src/lib/api`,
// and what it answers is covered by the Rust tests in `settings.rs`. What is
// worth testing here is what the screen does with each answer.
vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    checkSteamPath: vi.fn(),
    configPath: vi.fn(),
    logPath: vi.fn(),
    revealPath: vi.fn(),
    saveSettings: vi.fn(),
    resetSettings: vi.fn(),
  };
});

const { checkSteamPath, configPath, logPath, revealPath } = await import("../lib/api");
const { SettingsView } = await import("./SettingsView");

const check = vi.mocked(checkSteamPath);
const reveal = vi.mocked(revealPath);

const SETTINGS: Settings = { steamPathOverride: "", sampleIntervalMs: 2000 };

function renderWith(result: SteamPathCheck, settings: Partial<Settings> = {}) {
  check.mockResolvedValue(result);
  return render(
    <SettingsView settings={{ ...SETTINGS, ...settings }} onChange={() => {}} />,
  );
}

const steamField = () => screen.getByLabelText("Steam folder");

beforeEach(() => {
  check.mockReset();
  reveal.mockReset();
  vi.mocked(configPath).mockResolvedValue("C:\\AppData\\stasis\\settings.json");
  vi.mocked(logPath).mockResolvedValue("C:\\AppData\\stasis\\logs\\stasis.log");
});

describe("the Steam folder field", () => {
  it("says nothing about an empty field, because empty is the setting working", async () => {
    renderWith({ status: "automatic" });

    // The hint above the input already explains that empty means auto-detect.
    // A status repeating it would put a message under a field most people
    // never fill in.
    expect(await screen.findByRole("button", { name: "Restore defaults" })).toBeVisible();
    expect(steamField()).toHaveAttribute("aria-invalid", "false");
    expect(screen.queryByText(/clear the field/)).toBeNull();
  });

  it("tells a path that is not there apart from a folder without steamapps", async () => {
    renderWith({ status: "missing" }, { steamPathOverride: "D:\\Gone" });

    expect(await screen.findByText(/Nothing is at that path/)).toBeVisible();
    expect(screen.queryByText(/no steamapps directory/)).toBeNull();
  });

  it("says a real folder is simply not where Steam lives", async () => {
    // The unkind version of this is the old behaviour: the field accepted it,
    // and the user found out two screens later on a failed scan.
    renderWith({ status: "noSteamapps" }, { steamPathOverride: "D:\\Games" });

    const message = await screen.findByText(/no steamapps directory inside it/);
    expect(message).toHaveTextContent("steam.exe");
    expect(message).toHaveTextContent("clear the field");
  });

  it("calls a file a file rather than a folder missing something", async () => {
    renderWith({ status: "notAFolder" }, { steamPathOverride: "D:\\Steam\\steam.exe" });

    expect(await screen.findByText(/is a file, not a folder/)).toBeVisible();
  });

  it("confirms a folder that will work, rather than only reporting problems", async () => {
    renderWith({ status: "usable" }, { steamPathOverride: "D:\\Steam" });

    expect(await screen.findByText(/Found a steamapps folder here/)).toBeVisible();
    expect(steamField()).toHaveAttribute("aria-invalid", "false");
  });
});

describe("how the message reaches a screen reader", () => {
  it("associates the message with the input rather than only colouring it", async () => {
    renderWith({ status: "missing" }, { steamPathOverride: "D:\\Gone" });

    const message = await screen.findByText(/Nothing is at that path/);
    const describedBy = steamField().getAttribute("aria-describedby") ?? "";

    expect(message.id).not.toBe("");
    expect(describedBy.split(" ")).toContain(message.id);
  });

  it("marks the input invalid only when the folder is the thing that is wrong", async () => {
    // A check that could not run says so, but it is not evidence that what the
    // user typed is wrong, so the field is not marked invalid for it.
    check.mockRejectedValue(new Error("the disk went away"));
    render(
      <SettingsView
        settings={{ ...SETTINGS, steamPathOverride: "D:\\Steam" }}
        onChange={() => {}}
      />,
    );

    expect(await screen.findByText(/could not be checked/)).toHaveTextContent(
      "the disk went away",
    );
    expect(steamField()).toHaveAttribute("aria-invalid", "false");
  });
});

describe("when the check runs", () => {
  it("checks the stored path on arrival, before any scan is attempted", async () => {
    renderWith({ status: "missing" }, { steamPathOverride: "D:\\Gone" });

    await screen.findByText(/Nothing is at that path/);
    expect(check).toHaveBeenCalledWith("D:\\Gone");
  });

  it("checks again when the field is left", async () => {
    renderWith({ status: "usable" }, { steamPathOverride: "D:\\Steam" });
    await screen.findByText(/Found a steamapps folder here/);

    check.mockClear();
    check.mockResolvedValue({ status: "missing" });
    fireEvent.change(steamField(), { target: { value: "D:\\Typo" } });
    fireEvent.blur(steamField());

    expect(await screen.findByText(/Nothing is at that path/)).toBeVisible();
    expect(check).toHaveBeenCalledWith("D:\\Typo");
  });

  it("drops the old answer while the field is being edited", async () => {
    // A message about text that is no longer in the field is worse than no
    // message: it reads as a verdict on what the user is typing right now.
    renderWith({ status: "missing" }, { steamPathOverride: "D:\\Gone" });
    await screen.findByText(/Nothing is at that path/);

    fireEvent.change(steamField(), { target: { value: "D:\\Gon" } });

    expect(screen.queryByText(/Nothing is at that path/)).toBeNull();
  });
});

describe("the log file", () => {
  it("is named on screen so it can be found and attached to a bug report", async () => {
    renderWith({ status: "automatic" });

    expect(await screen.findByText("C:\\AppData\\stasis\\logs\\stasis.log")).toBeVisible();
  });

  it("opens the folder holding it, not the file itself", async () => {
    // Explorer needs a folder to open. The two file rows also have to be
    // distinguishable by name alone, since a screen reader lists the buttons
    // without the rows around them.
    renderWith({ status: "automatic" });

    const button = await screen.findByRole("button", {
      name: "Open the folder holding the log file",
    });
    fireEvent.click(button);

    expect(reveal).toHaveBeenCalledWith("C:\\AppData\\stasis\\logs");
  });
});
