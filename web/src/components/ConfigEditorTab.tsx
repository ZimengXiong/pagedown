import { useEffect, useMemo, useState } from "react";
import { parseSimpleToml, stringifySimpleToml, type ConfigTree } from "../config/tomlConfig";
import { sampleMarkdown } from "../fixtures/sample";
import { useDebouncedValue } from "../hooks/useDebouncedValue";
import { pdfBlobUrl, renderMarkdownPdf } from "../render/renderClient";
import { ConfigControls } from "./ConfigControls";
import { EditorPane } from "./EditorPane";
import { PdfPreview } from "./PdfPreview";

type ConfigEditorTabProps = {
  configToml: string;
  onConfigChange: (value: string) => void;
};

type ConfigMode = "controls" | "manual";

export function ConfigEditorTab({ configToml, onConfigChange }: ConfigEditorTabProps) {
  const [mode, setMode] = useState<ConfigMode>("controls");
  const [pdfUrl, setPdfUrl] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const debouncedConfig = useDebouncedValue(configToml, 450);

  const parsed = useMemo(() => {
    try {
      return { tree: parseSimpleToml(configToml), error: null as string | null };
    } catch (error) {
      return { tree: {} as ConfigTree, error: error instanceof Error ? error.message : String(error) };
    }
  }, [configToml]);

  useEffect(() => {
    setBusy(true);
    void renderMarkdownPdf({
      markdown: sampleMarkdown,
      configToml: debouncedConfig,
    })
      .then((result) => {
        setWarnings(parsed.error ? [parsed.error, ...result.warnings] : result.warnings);
        setPdfUrl((previous) => {
          if (previous) URL.revokeObjectURL(previous);
          return pdfBlobUrl(result.bytes);
        });
      })
      .finally(() => setBusy(false));
  }, [debouncedConfig, parsed.error]);

  return (
    <main>
      <nav aria-label="Config editor modes">
        <button type="button" aria-pressed={mode === "controls"} onClick={() => setMode("controls")}>
          Controls
        </button>
        <button type="button" aria-pressed={mode === "manual"} onClick={() => setMode("manual")}>
          TOML
        </button>
      </nav>
      <section aria-label="Config editor split view">
        {mode === "manual" ? (
          <EditorPane label="Render Config TOML" value={configToml} language="toml" onChange={onConfigChange} />
        ) : (
          <section aria-label="Render Config Controls">
            {parsed.error ? <p role="alert">{parsed.error}</p> : null}
            <ConfigControls
              tree={parsed.tree}
              onChange={(tree) => {
                onConfigChange(stringifySimpleToml(tree));
              }}
            />
          </section>
        )}
        <PdfPreview title="Sample PDF Preview" url={pdfUrl} busy={busy} warnings={warnings} zoom={100} />
      </section>
    </main>
  );
}
