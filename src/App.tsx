import { useCallback, useEffect, useState } from "react";

import { Rail, type ViewId } from "./components/Rail";
import { StatusBar } from "./components/StatusBar";
import { TitleBar } from "./components/TitleBar";
import {
  errorText,
  loadSettings,
  readFootprint,
  type Footprint,
  type Settings,
} from "./lib/api";
import { GamesView } from "./views/GamesView";
import { LibraryView } from "./views/LibraryView";
import { SettingsView } from "./views/SettingsView";

const FALLBACK_SETTINGS: Settings = {
  steamPathOverride: "",
  sampleIntervalMs: 2000,
};

export default function App() {
  const [view, setView] = useState<ViewId>("library");
  const [settings, setSettings] = useState<Settings>(FALLBACK_SETTINGS);
  const [footprint, setFootprint] = useState<Footprint | null>(null);
  const [footprintError, setFootprintError] = useState<string | null>(null);
  const [moduleCount, setModuleCount] = useState<number | null>(null);
  const [gameCount, setGameCount] = useState<number | null>(null);

  useEffect(() => {
    loadSettings()
      .then(setSettings)
      // A settings file that cannot be read should not take the app down; the
      // defaults are usable and Settings will show the real error on save.
      .catch(() => setSettings(FALLBACK_SETTINGS));
  }, []);

  useEffect(() => {
    let cancelled = false;

    const sample = async () => {
      try {
        const next = await readFootprint();
        if (cancelled) return;
        setFootprint(next);
        setFootprintError(null);
      } catch (error) {
        if (cancelled) return;
        setFootprintError(errorText(error));
      }
    };

    void sample();
    const timer = window.setInterval(() => void sample(), settings.sampleIntervalMs);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [settings.sampleIntervalMs]);

  // Memoised so the child effects that depend on them do not re-run on every
  // parent render, which would put the views in a reload loop.
  const handleModuleCount = useCallback((count: number | null) => {
    setModuleCount(count);
  }, []);
  const handleGameCount = useCallback((count: number | null) => {
    setGameCount(count);
  }, []);
  const goToSettings = useCallback(() => setView("settings"), []);

  return (
    <div className="shell">
      <TitleBar />

      <div className="body">
        <Rail
          active={view}
          onSelect={setView}
          moduleCount={moduleCount}
          gameCount={gameCount}
        />

        <main className="main">
          {view === "library" && <LibraryView onCountChange={handleModuleCount} />}
          {view === "games" && (
            <GamesView
              steamPathOverride={settings.steamPathOverride}
              onCountChange={handleGameCount}
              onGoToSettings={goToSettings}
            />
          )}
          {view === "settings" && (
            <SettingsView settings={settings} onChange={setSettings} />
          )}
        </main>
      </div>

      <StatusBar footprint={footprint} error={footprintError} />
    </div>
  );
}
