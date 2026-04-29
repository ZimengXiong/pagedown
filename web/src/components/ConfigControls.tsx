import type { ConfigControl, ConfigPrimitive, ConfigTree } from "../config/tomlConfig";
import { flattenControls, setConfigValue } from "../config/tomlConfig";

type ConfigControlsProps = {
  tree: ConfigTree;
  onChange: (tree: ConfigTree) => void;
};

export function ConfigControls({ tree, onChange }: ConfigControlsProps) {
  const controls = flattenControls(tree);
  const groups = groupControls(controls);

  return (
    <section aria-label="Config controls">
      {Object.entries(groups).map(([group, items]) => (
        <fieldset key={group}>
          <legend>{group}</legend>
          {items.map((control) => (
            <ConfigControlRow
              key={control.label}
              control={control}
              onChange={(value) => onChange(setConfigValue(tree, control.path, control.key, value))}
            />
          ))}
        </fieldset>
      ))}
    </section>
  );
}

function ConfigControlRow({
  control,
  onChange,
}: {
  control: ConfigControl;
  onChange: (value: ConfigPrimitive) => void;
}) {
  if (control.key === "math_mode") {
    return (
      <label>
        {control.label}
        <select value={String(control.value)} onChange={(event) => onChange(event.currentTarget.value)}>
          <option value="katex">katex</option>
          <option value="lualatex">lualatex</option>
          <option value="latex">latex</option>
          <option value="fallback">fallback</option>
        </select>
      </label>
    );
  }

  if (control.type === "boolean") {
    return (
      <label>
        <input
          type="checkbox"
          checked={Boolean(control.value)}
          onChange={(event) => onChange(event.currentTarget.checked)}
        />
        {control.label}
      </label>
    );
  }

  if (control.type === "number") {
    return (
      <label>
        {control.label}
        <input
          type="range"
          min={control.min}
          max={control.max}
          step={control.step}
          value={Number(control.value)}
          onChange={(event) => onChange(Number(event.currentTarget.value))}
        />
        <input
          type="number"
          step={control.step}
          value={Number(control.value)}
          onChange={(event) => onChange(Number(event.currentTarget.value))}
        />
      </label>
    );
  }

  return (
    <label>
      {control.label}
      <input type="text" value={String(control.value)} onChange={(event) => onChange(event.currentTarget.value)} />
    </label>
  );
}

function groupControls(controls: ConfigControl[]): Record<string, ConfigControl[]> {
  return controls.reduce<Record<string, ConfigControl[]>>((groups, control) => {
    const group = control.path.join(".") || "document";
    groups[group] = groups[group] ?? [];
    groups[group].push(control);
    return groups;
  }, {});
}
