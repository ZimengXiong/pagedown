import { useState } from "react";
import { MarkdownRendererTab } from "./components/MarkdownRendererTab";
import { ConfigEditorTab } from "./components/ConfigEditorTab";
import { sampleConfig, sampleMarkdown } from "./fixtures/sample";

type AppTab = "renderer" | "config";

export function App() {
  const [tab, setTab] = useState<AppTab>("renderer");
  const [markdown, setMarkdown] = useState(sampleMarkdown);
  const [configToml, setConfigToml] = useState(sampleConfig);

  return (
    <>
      <header>
        <h1>Native Markdown PDF</h1>
        <nav aria-label="Application tabs">
          <button type="button" aria-pressed={tab === "renderer"} onClick={() => setTab("renderer")}>
            Live Renderer
          </button>
          <button type="button" aria-pressed={tab === "config"} onClick={() => setTab("config")}>
            Config Editor
          </button>
        </nav>
      </header>
      {tab === "renderer" ? (
        <MarkdownRendererTab
          markdown={markdown}
          configToml={configToml}
          onMarkdownChange={setMarkdown}
          onConfigChange={setConfigToml}
        />
      ) : (
        <ConfigEditorTab configToml={configToml} onConfigChange={setConfigToml} />
      )}
    </>
  );
}
