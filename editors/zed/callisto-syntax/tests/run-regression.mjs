import path from "node:path";
import { execSync } from "node:child_process";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const grammarDir = path.join(root, "tree-sitter-callisto");
const fixturePath = path.join(root, "tests", "fixtures", "highlighting.cal");
const queryPath = path.join(root, "languages", "callisto", "highlights.scm");

function run(command, cwd = root) {
  return execSync(command, { cwd, encoding: "utf8" });
}

run("npx tree-sitter generate", grammarDir);

const parseOut = run(`npx tree-sitter parse ${fixturePath}`, grammarDir);
if (/\b(ERROR|MISSING)\b/.test(parseOut)) {
  throw new Error(`parse produced syntax error nodes:\n${parseOut}`);
}

const queryOut = run(`npx tree-sitter query ${queryPath} ${fixturePath}`, grammarDir);

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
