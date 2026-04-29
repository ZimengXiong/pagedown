import { PDFDocument, StandardFonts, rgb } from "pdf-lib";

export type RenderInput = {
  markdown: string;
  configToml: string;
  assets?: Record<string, Uint8Array>;
};

export type RenderResult = {
  bytes: Uint8Array;
  source: "wasm" | "placeholder";
  warnings: string[];
};

type WasmRenderer = {
  render_pdf?: (markdown: string, configToml: string, assetsJson?: string) => Uint8Array;
  renderPdf?: (markdown: string, configToml: string, assetsJson?: string) => Uint8Array;
};

declare global {
  interface Window {
    NativeMarkdownPdf?: WasmRenderer;
  }
}

export async function renderMarkdownPdf(input: RenderInput): Promise<RenderResult> {
  const wasm = window.NativeMarkdownPdf;
  const render = wasm?.render_pdf ?? wasm?.renderPdf;

  if (render) {
    const bytes = render(input.markdown, input.configToml, JSON.stringify(Object.keys(input.assets ?? {})));
    return {
      bytes,
      source: "wasm",
      warnings: [],
    };
  }

  const bytes = await placeholderPdf(input);
  return {
    bytes,
    source: "placeholder",
    warnings: ["WASM renderer not loaded. The UI is running with a static placeholder PDF."],
  };
}

async function placeholderPdf(input: RenderInput): Promise<Uint8Array> {
  const pdf = await PDFDocument.create();
  const page = pdf.addPage([612, 792]);
  const regular = await pdf.embedFont(StandardFonts.TimesRoman);
  const mono = await pdf.embedFont(StandardFonts.Courier);
  const titleFont = await pdf.embedFont(StandardFonts.HelveticaBold);

  page.drawText("Native Markdown PDF Web UI", {
    x: 72,
    y: 720,
    size: 24,
    font: titleFont,
    color: rgb(0.1, 0.12, 0.16),
  });
  page.drawText("The WASM renderer adapter is ready, but no renderer module is loaded yet.", {
    x: 72,
    y: 684,
    size: 12,
    font: regular,
    color: rgb(0.1, 0.12, 0.16),
  });
  page.drawText("Expected browser API:", {
    x: 72,
    y: 652,
    size: 11,
    font: titleFont,
  });
  page.drawText("window.NativeMarkdownPdf.render_pdf(markdown, configToml, assetsJson) -> Uint8Array", {
    x: 72,
    y: 632,
    size: 9,
    font: mono,
  });

  const markdownLines = input.markdown.split(/\r?\n/).slice(0, 22);
  let y = 590;
  page.drawText("Markdown preview input:", { x: 72, y, size: 11, font: titleFont });
  y -= 18;
  for (const line of markdownLines) {
    page.drawText(line.slice(0, 92), {
      x: 72,
      y,
      size: 9,
      font: mono,
      color: rgb(0.16, 0.18, 0.22),
    });
    y -= 13;
  }

  return pdf.save();
}

export function pdfBlobUrl(bytes: Uint8Array): string {
  return URL.createObjectURL(new Blob([bytesToArrayBuffer(bytes)], { type: "application/pdf" }));
}

function bytesToArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
}
