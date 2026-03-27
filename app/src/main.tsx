import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { HttpAdapter, LegionProvider } from "./services";
import "./index.css";

const adapter = new HttpAdapter();

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <LegionProvider adapter={adapter}>
        <App />
      </LegionProvider>
    </StrictMode>,
  );
}
