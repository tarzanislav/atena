// Writes the licence texts of every part the desktop app is built from into
// src-tauri/resources/notices/, which the installer carries: the Rust crates that end up in the
// program (for the given target), the JavaScript packages of the interface (with the fonts and
// icons), ATENA's own licence and PostgreSQL's.
//
//   node scripts/notices.mjs [--target x86_64-pc-windows-msvc]

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

// in CI, every failure becomes an annotation of the pull request
process.on("uncaughtException", (e) => {
  console.log(`::error::notices: ${e.message.split("\n")[0]}`);
  process.exit(1);
});

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const out = join(root, "src-tauri", "resources", "notices");
const target = process.argv.includes("--target") ? process.argv[process.argv.indexOf("--target") + 1] : null;

const LICENCE_FILE = /^(licen[cs]e|copying|notice|copyright|authors)([-._].*)?$/i;

function licenceTexts(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((name) => LICENCE_FILE.test(name))
    .sort()
    .map((name) => ({ name, text: readFileSync(join(dir, name), "utf8").trim() }));
}

function section(title, licence, url, texts) {
  const lines = [`${"=".repeat(78)}`, title, `Licence: ${licence || "see the texts below"}`];
  if (url) lines.push(`Source: ${url}`);
  lines.push("=".repeat(78));
  for (const t of texts) lines.push("", `--- ${t.name} ---`, "", t.text);
  if (texts.length === 0) lines.push("", "(The package carries no licence file; its licence is named above.)");
  return lines.join("\n");
}

/** The crates the program is built from: normal dependencies of atena-desktop, for `target`. */
function rustNotices() {
  const args = ["metadata", "--format-version", "1", "--locked", "--manifest-path", join(root, "src-tauri", "Cargo.toml")];
  if (target) args.push("--filter-platform", target);
  const meta = JSON.parse(execFileSync("cargo", args, { maxBuffer: 256 * 1024 * 1024 }).toString());
  const packages = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const seen = new Set();
  const queue = [meta.resolve.root];
  while (queue.length) {
    const id = queue.pop();
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dep of nodes.get(id)?.deps ?? []) {
      if (dep.dep_kinds.some((k) => k.kind === null)) queue.push(dep.pkg);
    }
  }
  const crates = [...seen]
    .map((id) => packages.get(id))
    // ATENA's own crates are covered by its licence
    .filter((p) => p && p.source)
    .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
  const body = crates.map((p) =>
    section(`${p.name} ${p.version}`, p.license, p.repository, licenceTexts(dirname(p.manifest_path))),
  );
  return { count: crates.length, text: body.join("\n\n") };
}

/** The packages of the interface: everything that is not a development tool. */
function javascriptNotices() {
  const lock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
  const packages = Object.entries(lock.packages)
    .filter(([path, p]) => path && !p.dev)
    .map(([path, p]) => ({ path, name: path.replace(/^.*node_modules\//, ""), ...p }))
    .sort((a, b) => a.name.localeCompare(b.name));
  const body = packages.map((p) =>
    section(`${p.name} ${p.version}`, p.license, `https://www.npmjs.com/package/${p.name}`, licenceTexts(join(root, p.path))),
  );
  return { count: packages.length, text: body.join("\n\n") };
}

mkdirSync(out, { recursive: true });
const rust = rustNotices();
writeFileSync(join(out, "rust-crates.txt"), `Rust crates in the ATENA desktop app (${rust.count})\n\n${rust.text}\n`);
const js = javascriptNotices();
writeFileSync(join(out, "interface-packages.txt"), `Packages of the ATENA interface, fonts and icons included (${js.count})\n\n${js.text}\n`);
copyFileSync(join(root, "..", "..", "LICENSE"), join(out, "ATENA-LICENSE.txt"));
copyFileSync(join(root, "notices", "postgresql.txt"), join(out, "postgresql.txt"));
console.log(`notices: ${rust.count} crates, ${js.count} packages → ${out}`);
