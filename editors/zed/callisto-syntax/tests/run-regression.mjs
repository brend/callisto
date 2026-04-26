import path from "node:path";
import fs from "node:fs";
import { execFileSync } from "node:child_process";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const grammarDir = path.join(root, "grammars", "callisto");
const fixturePath = path.join(root, "tests", "fixtures", "highlighting.cal");
const queryPath = path.join(root, "languages", "callisto", "highlights.scm");
const npx = findExecutable("npx");

function findExecutable(name) {
  for (const dir of (process.env.PATH ?? "").split(path.delimiter)) {
    const candidate = path.join(dir, name);
    if (fs.existsSync(candidate)) return candidate;
  }
  throw new Error(`${name} was not found on PATH`);
}

function run(command, args, cwd = root) {
  return execFileSync(command, args, { cwd, encoding: "utf8" });
}

function runNpx(args, cwd = root) {
  return run(process.execPath, [npx, ...args], cwd);
}

runNpx(["tree-sitter", "generate"], grammarDir);

const parseOut = runNpx(["tree-sitter", "parse", fixturePath], grammarDir);
if (/\b(ERROR|MISSING)\b/.test(parseOut)) {
  throw new Error(`parse produced syntax error nodes:\n${parseOut}`);
}

const queryOut = runNpx(["tree-sitter", "query", queryPath, fixturePath], grammarDir);

const checks = [
  { name: "interpolation punctuation", pattern: /punctuation\.special[\s\S]*\$\{/ },
  { name: "escaped interpolation marker", pattern: /string\.escape[\s\S]*\\\$/ },
  { name: "function capture", pattern: /function[\s\S]*`length`/ },
  { name: "method call capture", pattern: /function\.method[\s\S]*`normalize`/ },
  { name: "builtin type capture", pattern: /type\.builtin[\s\S]*`String`/ },
  { name: "keyword capture", pattern: /keyword[\s\S]*`fn`/ },
];

for (const check of checks) {
  if (!check.pattern.test(queryOut)) {
    throw new Error(`missing ${check.name} in query output`);
  }
}

console.log("Zed Callisto grammar regression checks passed.");
