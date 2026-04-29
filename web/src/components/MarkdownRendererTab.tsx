import { useEffect, useMemo, useState } from "react";
import { sampleConfig, sampleMarkdown } from "../fixtures/sample";
import { useDebouncedValue } from "../hooks/useDebouncedValue";
import { pdfBlobUrl, renderMarkdownPdf } from "../render/renderClient";
import { EditorPane } from "./EditorPane";
import { PdfPreview } from "./PdfPreview";
import { Toolbar } from "./Toolbar";

type MarkdownRendererTabProps = {
  markdown: string;
  configToml: string;
  onMarkdownChange: (value: string) => void;
  onConfigChange: (value: string) => void;
};

export function MarkdownRendererTab({
  markdown,
  configToml,
  onMarkdownChange,
  onConfigChange,
}: MarkdownRendererTabProps) {
  const [pdfUrl, setPdfUrl] = useState<string | null>(null);
  const [pdfBytes, setPdfBytes] = useState<Uint8Array | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [zoom, setZoom] = useState(100);
  const debouncedMarkdown = useDebouncedValue(markdown, 450);
  const debouncedConfig = useDebouncedValue(configToml, 450);

  const render = useMemo(
    () => async () => {
      setBusy(true);
      try {
        const result = await renderMarkdownPdf({
          markdown,
          configToml,
        });
        setPdfBytes(result.bytes);
        setWarnings(result.warnings);
        setPdfUrl((previous) => {
          if (previous) URL.revokeObjectURL(previous);
          return pdfBlobUrl(result.bytes);
        });
      } finally {
        setBusy(false);
      }
    },
    [markdown, configToml],
  );

  useEffect(() => {
    void renderMarkdownPdf({
      markdown: debouncedMarkdown,
      configToml: debouncedConfig,
    }).then((result) => {
      setPdfBytes(result.bytes);
      setWarnings(result.warnings);
      setPdfUrl((previous) => {
        if (previous) URL.revokeObjectURL(previous);
        return pdfBlobUrl(result.bytes);
      });
    });
  }, [debouncedMarkdown, debouncedConfig]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const mod = event.metaKey || event.ctrlKey;
      if (mod && event.key === "Enter") {
        event.preventDefault();
        void render();
      }
      if (mod && event.key.toLowerCase() === "s") {
        event.preventDefault();
        downloadText("document.md", markdown);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [markdown, render]);

  return (
    <main>
      <Toolbar
        onRender={() => void render()}
        onOpenMarkdown={(file) => {
          void file.text().then(onMarkdownChange);
        }}
        onDownloadMarkdown={() => downloadText("document.md", markdown)}
        onDownloadPdf={() => {
          if (pdfBytes) downloadBytes("document.pdf", pdfBytes, "application/pdf");
        }}
        onReset={() => {
          onMarkdownChange(sampleMarkdown);
          onConfigChange(sampleConfig);
        }}
        zoom={zoom}
        onZoom={setZoom}
        disabledPdf={!pdfBytes}
      />
      <section aria-label="Live renderer split view">
        <EditorPane label="Markdown" value={markdown} language="markdown" onChange={onMarkdownChange} />
        <PdfPreview title="PDF Preview" url={pdfUrl} busy={busy} warnings={warnings} zoom={zoom} />
      </section>
    </main>
  );
}

function downloadText(filename: string, text: string) {
  downloadBytes(filename, new TextEncoder().encode(text), "text/plain;charset=utf-8");
}

function downloadBytes(filename: string, bytes: Uint8Array, type: string) {
  const data = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
  const url = URL.createObjectURL(new Blob([data], { type }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}
