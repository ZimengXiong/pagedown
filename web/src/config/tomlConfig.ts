export type ConfigPrimitive = string | number | boolean;
export type ConfigValue = ConfigPrimitive | ConfigTree;
export type ConfigTree = { [key: string]: ConfigValue };
export type ConfigControl = {
  path: string[];
  key: string;
  label: string;
  value: ConfigPrimitive;
  type: "number" | "boolean" | "string";
  min?: number;
  max?: number;
  step?: number;
};

export function parseSimpleToml(input: string): ConfigTree {
  const root: ConfigTree = {};
  let currentPath: string[] = [];

  for (const rawLine of input.split(/\r?\n/)) {
    const line = stripComment(rawLine).trim();
    if (!line) continue;

    const tableMatch = line.match(/^\[([^\]]+)\]$/);
    if (tableMatch) {
      currentPath = tableMatch[1].split(".").map((part) => part.trim()).filter(Boolean);
      ensureTable(root, currentPath);
      continue;
    }

    const equals = line.indexOf("=");
    if (equals < 0) continue;

    const key = line.slice(0, equals).trim();
    const value = parseValue(line.slice(equals + 1).trim());
    const table = ensureTable(root, currentPath);
    table[key] = value;
  }

  return root;
}

export function stringifySimpleToml(tree: ConfigTree): string {
  const topLevel: string[] = [];
  const tables: string[] = [];

  for (const [key, value] of Object.entries(tree)) {
    if (isTree(value)) {
      writeTable(tables, [key], value);
    } else {
      topLevel.push(`${key} = ${formatValue(value)}`);
    }
  }

  return [...topLevel, "", ...tables].join("\n").trimEnd() + "\n";
}

export function flattenControls(tree: ConfigTree): ConfigControl[] {
  const out: ConfigControl[] = [];

  function visit(node: ConfigTree, path: string[]) {
    for (const [key, value] of Object.entries(node)) {
      if (isTree(value)) {
        visit(value, [...path, key]);
      } else {
        out.push({
          path,
          key,
          label: [...path, key].join("."),
          value,
          type: typeof value === "number" ? "number" : typeof value === "boolean" ? "boolean" : "string",
          ...numberBounds(key, value),
        });
      }
    }
  }

  visit(tree, []);
  return out;
}

export function setConfigValue(tree: ConfigTree, path: string[], key: string, value: ConfigPrimitive): ConfigTree {
  const clone = structuredClone(tree) as ConfigTree;
  const table = ensureTable(clone, path);
  table[key] = value;
  return clone;
}

function writeTable(out: string[], path: string[], node: ConfigTree) {
  out.push(`[${path.join(".")}]`);
  for (const [key, value] of Object.entries(node)) {
    if (!isTree(value)) {
      out.push(`${key} = ${formatValue(value)}`);
    }
  }
  out.push("");
  for (const [key, value] of Object.entries(node)) {
    if (isTree(value)) {
      writeTable(out, [...path, key], value);
    }
  }
}

function stripComment(line: string): string {
  let quoted = false;
  for (let index = 0; index < line.length; index += 1) {
    const ch = line[index];
    if (ch === '"' && line[index - 1] !== "\\") quoted = !quoted;
    if (ch === "#" && !quoted) return line.slice(0, index);
  }
  return line;
}

function parseValue(value: string): ConfigPrimitive {
  if (value === "true") return true;
  if (value === "false") return false;
  if (value.startsWith('"') && value.endsWith('"')) {
    return value.slice(1, -1).replace(/\\"/g, '"');
  }
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : value;
}

function formatValue(value: ConfigPrimitive): string {
  if (typeof value === "string") return `"${value.replace(/"/g, '\\"')}"`;
  if (typeof value === "boolean") return value ? "true" : "false";
  return Number.isInteger(value) ? value.toString() : value.toFixed(3).replace(/0+$/, "").replace(/\.$/, ".0");
}

function ensureTable(root: ConfigTree, path: string[]): ConfigTree {
  let table = root;
  for (const part of path) {
    const value = table[part];
    if (!isTree(value)) table[part] = {};
    table = table[part] as ConfigTree;
  }
  return table;
}

function isTree(value: ConfigValue | undefined): value is ConfigTree {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function numberBounds(key: string, value: ConfigPrimitive): Partial<ConfigControl> {
  if (typeof value !== "number") return {};
  const step = key.endsWith("_multiplier") ? 0.01 : key.includes("max_rows") || key.includes("min_lines") ? 1 : 0.1;
  const min = key.includes("offset") || key.includes("shift") ? -100 : 0;
  const max = key.includes("page_width") || key.includes("page_height") ? 1200 : key.includes("keep") ? 400 : 160;
  return { min, max: Math.max(max, value * 2, min + step), step };
}
