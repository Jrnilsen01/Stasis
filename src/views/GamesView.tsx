import { useCallback, useEffect, useState } from "react";

import { errorText, scanSteamGames, type ScanResult } from "../lib/api";
import { formatBytes, formatDate } from "../lib/format";

type GamesViewProps = {
  steamPathOverride: string;
  onCountChange: (count: number | null) => void;
  onGoToSettings: () => void;
};

type ScanState =
  | { phase: "scanning" }
  | { phase: "failed"; message: string }
  | { phase: "done"; result: ScanResult };

/**
 * Installed games, read from Steam's own manifests on disk.
 *
 * This is the one screen with real rows on a fresh install, so its composition
 * is deliberately unlike the Modules screen: a dense left-aligned list under a
 * toolbar of facts, rather than a centred block. That difference is the RHYTHM
 * dial (2) doing its job. Sections vary because their content differs, not
 * because variety was applied on top.
 *
 * Nothing here is estimated. A game with no recorded play time says "never
 * played" rather than showing a plausible date.
 */
export function GamesView({
  steamPathOverride,
  onCountChange,
  onGoToSettings,
}: GamesViewProps) {
  const [state, setState] = useState<ScanState>({ phase: "scanning" });

  const scan = useCallback(async () => {
    setState({ phase: "scanning" });
    try {
      const result = await scanSteamGames(steamPathOverride);
      setState({ phase: "done", result });
      onCountChange(result.status === "scanned" ? result.games.length : null);
    } catch (error) {
      setState({ phase: "failed", message: errorText(error) });
      onCountChange(null);
    }
  }, [steamPathOverride, onCountChange]);

  useEffect(() => {
    void scan();
  }, [scan]);

  if (state.phase === "scanning") {
    return (
      <div className="state">
        <p className="state__label">SCANNING</p>
        <h2 className="state__title">Reading Steam library folders</h2>
        <p className="state__body">
          Checking the registry for the Steam install, then reading each
          library folder it lists. Local files only, no network.
        </p>
      </div>
    );
  }

  if (state.phase === "failed") {
    return (
      <div className="state">
        <p className="state__label state__label--error">FAILED</p>
        <h2 className="state__title">The scan could not run</h2>
        <p className="state__body">{state.message}</p>
        <div className="state__actions">
          <button type="button" className="btn btn--primary" onClick={() => void scan()}>
            Run the scan again
          </button>
        </div>
      </div>
    );
  }

  const { result } = state;

  if (result.status === "steamNotFound") {
    return (
      <div className="state">
        <p className="state__label">NOT FOUND</p>
        <h2 className="state__title">
          {result.overridden
            ? "That folder does not hold a Steam library"
            : "Steam is not where it was expected"}
        </h2>
        <p className="state__body">
          {result.overridden
            ? "The Steam folder is set by hand in Settings, so only that folder was checked. It has no steamapps directory inside it. Correct the path, or clear the field to go back to detecting Steam automatically."
            : "These are the locations that were checked, in order. If Steam lives somewhere else, set the folder in Settings and the scan will use it."}
        </p>
        <ul className="state__list">
          {result.searched.map((path) => (
            <li key={path}>{path}</li>
          ))}
        </ul>
        <div className="state__actions">
          <button type="button" className="btn btn--primary" onClick={onGoToSettings}>
            Set the Steam folder
          </button>
          <button type="button" className="btn" onClick={() => void scan()}>
            Look again
          </button>
        </div>
      </div>
    );
  }

  if (result.status === "unreadable") {
    return (
      <div className="state">
        <p className="state__label state__label--error">UNREADABLE</p>
        <h2 className="state__title">Steam was found but could not be read</h2>
        <p className="state__body">
          {result.path} exists, but reading it failed: {result.reason}
        </p>
        <div className="state__actions">
          <button type="button" className="btn btn--primary" onClick={() => void scan()}>
            Try reading again
          </button>
          <button type="button" className="btn" onClick={onGoToSettings}>
            Point at a different folder
          </button>
        </div>
      </div>
    );
  }

  if (result.games.length === 0) {
    return (
      <div className="state">
        <p className="state__label">NO GAMES</p>
        <h2 className="state__title">Steam is installed, with nothing in it</h2>
        <p className="state__body">
          Read {result.steamRoot} and {result.libraries.length} library folder
          {result.libraries.length === 1 ? "" : "s"}, and found no installed
          games. Installing a game and scanning again will fill this list.
        </p>
        <div className="state__actions">
          <button type="button" className="btn btn--primary" onClick={() => void scan()}>
            Scan again
          </button>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="toolbar">
        <span className="toolbar__fact">
          {result.games.length} game{result.games.length === 1 ? "" : "s"}
          {result.libraries.length === 1
            ? ` in ${result.libraries[0]}`
            : ` across ${result.libraries.length} libraries`}
        </span>
        <div className="toolbar__spacer" />
        <button type="button" className="btn" onClick={() => void scan()}>
          Scan again
        </button>
      </div>
      <ul className="rows">
        {result.games.map((game) => (
          <li key={`${game.library}-${game.appId}`} className="row">
            <span className="row__name">{game.name}</span>
            <span className="row__meta">
              app {game.appId} · {formatDate(game.lastPlayed)}
              {/* The library path only earns its place when there is more than
                  one: repeating the same folder on every row is noise, and the
                  toolbar already says which library was read. */}
              {result.libraries.length > 1 && <> · {game.library}</>}
            </span>
            <span className="row__figure">{formatBytes(game.sizeOnDisk)}</span>
          </li>
        ))}
      </ul>
    </>
  );
}
