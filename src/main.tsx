import React from "react";
import ReactDOM from "react-dom/client";

// Fonts are bundled rather than fetched from a CDN: this is a desktop app that
// has to look right with no network, and a fallback face would break the
// condensed labels the layout is measured against.
import "@fontsource/barlow/400.css";
import "@fontsource/barlow/500.css";
import "@fontsource/barlow/600.css";
import "@fontsource/barlow-condensed/500.css";
import "@fontsource/barlow-condensed/600.css";

import "./styles/tokens.css";
import "./styles/app.css";

import App from "./App";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
