type ToolbarProps = {
  onRender: () => void;
  onOpenMarkdown: (file: File) => void;
  onDownloadMarkdown: () => void;
  onDownloadPdf: () => void;
  onReset: () => void;
  zoom: number;
  onZoom: (zoom: number) => void;
  disabledPdf: boolean;
};

export function Toolbar({
  onRender,
  onOpenMarkdown,
  onDownloadMarkdown,
  onDownloadPdf,
  onReset,
  zoom,
  onZoom,
  disabledPdf,
}: ToolbarProps) {
  return (
    <nav aria-label="Document tools">
      <button type="button" onClick={onRender}>
        Render
      </button>
      <label>
        Open Markdown
        <input
          type="file"
          accept=".md,.markdown,text/markdown,text/plain"
          onChange={(event) => {
            const file = event.currentTarget.files?.[0];
            if (file) onOpenMarkdown(file);
            event.currentTarget.value = "";
          }}
        />
      </label>
      <button type="button" onClick={onDownloadMarkdown}>
        Save Markdown
      </button>
      <button type="button" onClick={onDownloadPdf} disabled={disabledPdf}>
        Export PDF
      </button>
      <button type="button" onClick={onReset}>
        Reset Sample
      </button>
      <label>
        Zoom
        <input
          type="number"
          min="25"
          max="300"
          step="5"
          value={zoom}
          onChange={(event) => onZoom(Number(event.currentTarget.value))}
        />
      </label>
    </nav>
  );
}
