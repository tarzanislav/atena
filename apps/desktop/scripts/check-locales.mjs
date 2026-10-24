// Every locale has exactly the messages of the Slovak reference, each file parses without errors,
// and every error code of the backend (crates/atena-app/src/error.rs) has its text.
// A release is blocked while a core language lacks a message (docs/INTERNATIONALIZATION.md).

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { FluentBundle, FluentResource } from "@fluent/bundle";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const languages = JSON.parse(readFileSync(join(root, "locales", "index.json"), "utf8"));
const REFERENCE = "sk";

/** Message identifiers of a file, in order, and the problems found while reading it. */
function read(code) {
  let text;
  try {
    text = readFileSync(join(root, "locales", `${code}.ftl`), "utf8");
  } catch (e) {
    return { ids: [], problems: [`cannot be read: ${e.message}`] };
  }
  const problems = [];
  const ids = [];
  const lines = text.split("\n");
  lines.forEach((line, n) => {
    const m = /^([a-zA-Z][a-zA-Z0-9_-]*)\s*=/.exec(line);
    if (m) ids.push(m[1]);
    else if (/^\S/.test(line) && !line.startsWith("#")) problems.push(`line ${n + 1}: not a message: ${line}`);
  });
  const bundle = new FluentBundle(code, { useIsolating: false });
  for (const error of bundle.addResource(new FluentResource(text))) problems.push(error.message);
  for (const id of ids) {
    const message = bundle.getMessage(id);
    if (!message?.value) problems.push(`${id}: no value`);
    else {
      const errors = [];
      bundle.formatPattern(message.value, { count: 2, checks: 2, time: "", date: "" }, errors);
      // a missing variable is fine here (each message gets its own); anything else is not
      for (const e of errors) if (!(e instanceof ReferenceError)) problems.push(`${id}: ${e.message}`);
    }
  }
  const seen = new Set();
  for (const id of ids) {
    if (seen.has(id)) problems.push(`${id}: defined twice`);
    seen.add(id);
  }
  return { ids, problems };
}

let failed = false;
const fail = (message) => {
  console.error(message);
  failed = true;
};

const reference = read(REFERENCE);
const referenceIds = new Set(reference.ids);
for (const { code } of languages) {
  const { ids, problems } = code === REFERENCE ? reference : read(code);
  for (const p of problems) fail(`${code}.ftl: ${p}`);
  const own = new Set(ids);
  for (const id of referenceIds) if (!own.has(id)) fail(`${code}.ftl: missing ${id}`);
  for (const id of own) if (!referenceIds.has(id)) fail(`${code}.ftl: ${id} is not in ${REFERENCE}.ftl`);
}

// every error code of the backend needs a text
const errors = readFileSync(join(root, "..", "..", "crates", "atena-app", "src", "error.rs"), "utf8");
const codes = /pub const CODES: &\[&str\] = &\[([^\]]*)\]/.exec(errors);
if (!codes) fail("error.rs: CODES not found");
else
  for (const code of codes[1].matchAll(/"([a-z-]+)"/g))
    if (!referenceIds.has(`error-${code[1]}`)) fail(`${REFERENCE}.ftl: missing error-${code[1]}`);

// every message the interface names literally exists
function sources(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? sources(path) : /\.tsx?$/.test(name) ? [path] : [];
  });
}
for (const file of sources(join(root, "src"))) {
  const text = readFileSync(file, "utf8");
  for (const m of text.matchAll(/\bt\("([a-z0-9-]+)"/g))
    if (!referenceIds.has(m[1])) fail(`${file}: unknown message ${m[1]}`);
}

if (failed) process.exit(1);
console.log(`locales complete: ${languages.map((l) => l.code).join(", ")} (${referenceIds.size} messages each)`);
