import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    listModules: vi.fn(),
    modulesDir: vi.fn(),
    revealPath: vi.fn(),
  };
});

const { listModules, modulesDir } = await import("../lib/api");
const { LibraryView } = await import("./LibraryView");

const list = vi.mocked(listModules);
const dir = vi.mocked(modulesDir);

beforeEach(() => {
  list.mockReset();
  dir.mockReset();
});

describe("modules folder read failure", () => {
  // Implemented but never triggered before this test. The Rust side returns
  // this string when the folder cannot be listed, or when a module.json is
  // malformed, which is the case the Rust tests cover.
  const failure = "C:\\Users\\me\\AppData\\Roaming\\Stasis\\modules is not a valid module manifest";

  it("shows the failure rather than an empty list", async () => {
    // The distinction that matters: an empty list says "you have no modules",
    // which would be a false statement when the folder could not be read.
    list.mockRejectedValue(new Error(failure));
    dir.mockResolvedValue("C:\\Users\\me\\AppData\\Roaming\\Stasis\\modules");

    render(<LibraryView onCountChange={() => {}} />);

    expect(await screen.findByText("The modules folder could not be read")).toBeVisible();
    expect(screen.getByText("FAILED")).toBeVisible();
    expect(screen.queryByText("No modules yet")).not.toBeInTheDocument();
  });

  it("surfaces the reason it was given", async () => {
    list.mockRejectedValue(new Error(failure));
    dir.mockResolvedValue("C:\\modules");

    render(<LibraryView onCountChange={() => {}} />);

    expect(await screen.findByText(failure)).toBeVisible();
  });

  it("offers a retry", async () => {
    list.mockRejectedValue(new Error(failure));
    dir.mockResolvedValue("C:\\modules");

    render(<LibraryView onCountChange={() => {}} />);

    expect(await screen.findByRole("button", { name: "Try reading again" })).toBeEnabled();
  });

  it("does not offer to open a folder it never learned the path of", async () => {
    // Both calls run through one Promise.all, so a rejection can leave the
    // folder unknown. A button with no path would be a dead control.
    list.mockRejectedValue(new Error(failure));
    dir.mockRejectedValue(new Error("no data directory available"));

    render(<LibraryView onCountChange={() => {}} />);

    await screen.findByText("The modules folder could not be read");
    expect(screen.queryByRole("button", { name: "Open the folder" })).not.toBeInTheDocument();
  });

  it("reports a null count rather than zero", async () => {
    const onCountChange = vi.fn();
    list.mockRejectedValue(new Error(failure));
    dir.mockResolvedValue("C:\\modules");

    render(<LibraryView onCountChange={onCountChange} />);

    await screen.findByText("The modules folder could not be read");
    expect(onCountChange).toHaveBeenCalledWith(null);
    expect(onCountChange).not.toHaveBeenCalledWith(0);
  });
});

describe("empty modules folder", () => {
  it("is a success state, not a failure", async () => {
    // The fresh-install case, and the screen most people will see first.
    list.mockResolvedValue([]);
    dir.mockResolvedValue("C:\\modules");

    render(<LibraryView onCountChange={() => {}} />);

    expect(await screen.findByText("No modules yet")).toBeVisible();
    expect(screen.queryByText("FAILED")).not.toBeInTheDocument();
  });
});
