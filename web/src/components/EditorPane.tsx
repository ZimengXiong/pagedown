import CodeMirror from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { StreamLanguage } from "@codemirror/language";
import { toml } from "@codemirror/legacy-modes/mode/toml";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { searchKeymap } from "@codemirror/search";
import { keymap, lineNumbers } from "@codemirror/view";

type EditorPaneProps = {
  label: string;
  value: string;
  language: "markdown" | "toml";
  onChange: (value: string) => void;
};

export function EditorPane({ label, value, language, onChange }: EditorPaneProps) {
  return (
    <section aria-label={label}>
      <header>
        <h2>{label}</h2>
      </header>
      <CodeMirror
        value={value}
        minHeight="70vh"
        basicSetup={false}
        extensions={[
          lineNumbers(),
          history(),
          language === "markdown" ? markdown() : StreamLanguage.define(toml),
          keymap.of([indentWithTab, ...defaultKeymap, ...historyKeymap, ...searchKeymap]),
        ]}
        onChange={onChange}
      />
    </section>
  );
}
