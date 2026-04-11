import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const grammarPath = path.join(root, "syntaxes", "callisto.tmLanguage.json");
const fixturePath = path.join(root, "tests", "fixtures", "highlighting.cal");
const expectationsPath = path.join(root, "tests", "expectations.json");

const grammar = JSON.parse(fs.readFileSync(grammarPath, "utf8"));
const fixtureLines = fs
  .readFileSync(fixturePath, "utf8")
  .split(/\r?\n/)
  .map((line) => line.trimEnd());
const expectations = JSON.parse(fs.readFileSync(expectationsPath, "utf8"));

function findPatternByName(node, name) {
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = findPatternByName(child, name);
      if (found) return found;
    }
    return null;
  }
  if (!node || typeof node !== "object") return null;
  if (node.name === name && typeof node.match === "string") return node;
  for (const child of Object.values(node)) {
    const found = findPatternByName(child, name);
    if (found) return found;
  }
  return null;
}

function findFixtureLine(sample) {
  const line = fixtureLines.find((candidate) => candidate.includes(sample));
  if (!line) {
    throw new Error(`fixture line containing '${sample}' was not found`);
  }
  return line;
}

function nthMatch(regex, input, n) {
  let seen = 0;
  let match;
  while ((match = regex.exec(input)) !== null) {
    seen += 1;
    if (seen === n) return match;
    if (match[0] === "") regex.lastIndex += 1;
  }
  return null;
}

let failures = 0;
for (const test of expectations) {
  const pattern = findPatternByName(grammar, test.scope);
  if (!pattern) {
    console.error(`FAIL ${test.name}: scope '${test.scope}' not found`);
    failures += 1;
    continue;
  }

  const line = findFixtureLine(test.sample);
  const occurrence = Number.isInteger(test.occurrence) ? test.occurrence : 1;
  const regex = new RegExp(pattern.match, "g");
  const match = nthMatch(regex, line, occurrence);

  if (!match) {
    console.error(
      `FAIL ${test.name}: no match for scope '${test.scope}' on line '${line}'`
    );
    failures += 1;
    continue;
  }

  let localFailure = false;
  for (const [group, expected] of Object.entries(test.captures)) {
    const index = Number.parseInt(group, 10);
    const actual = match[index];
    if (actual !== expected) {
      console.error(
        `FAIL ${test.name}: capture ${group} expected '${expected}', got '${actual}'`
      );
      failures += 1;
      localFailure = true;
    }
  }

  if (!localFailure) {
    console.log(`PASS ${test.name}`);
  }
}

if (failures > 0) {
  process.exit(1);
}

console.log(`All ${expectations.length} grammar regression checks passed.`);
