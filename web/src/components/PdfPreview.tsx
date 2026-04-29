type PdfPreviewProps = {
  title: string;
  url: string | null;
  busy: boolean;
  warnings: string[];
  zoom: number;
};

export function PdfPreview({ title, url, busy, warnings, zoom }: PdfPreviewProps) {
  return (
    <section aria-label={title}>
      <header>
        <h2>{title}</h2>
        {busy ? <p role="status">Rendering...</p> : null}
        {warnings.length > 0 ? (
          <details open>
            <summary>Warnings</summary>
            <ul>
              {warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          </details>
        ) : null}
      </header>
      {url ? (
        <iframe title={title} src={`${url}#zoom=${zoom}`} width="100%" height="900" />
      ) : (
        <p>No PDF rendered yet.</p>
      )}
    </section>
  );
}
