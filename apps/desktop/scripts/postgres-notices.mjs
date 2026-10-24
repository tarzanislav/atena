// Lists the libraries that come with the bundled PostgreSQL programs and appends their licences to
// src-tauri/resources/notices/postgresql.txt. Every library must be known here: an unknown one
// stops the build until a person has checked its licence (pillar 4: no law may be broken).
//
//   node scripts/postgres-notices.mjs src-tauri/resources/postgres

import { appendFileSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const postgres = process.argv[2] ?? join(root, "src-tauri", "resources", "postgres");
const notices = join(root, "src-tauri", "resources", "notices", "postgresql.txt");

// the canonical licence texts, read from their projects at build time
const TEXTS = {
  apache: { file: join(root, "..", "..", "LICENSE") },
  lgpl: { url: "https://www.gnu.org/licenses/old-licenses/lgpl-2.1.txt" },
  icu: { url: "https://raw.githubusercontent.com/unicode-org/icu/main/LICENSE" },
  libxml2: { url: "https://gitlab.gnome.org/GNOME/libxml2/-/raw/master/Copyright" },
  libxslt: { url: "https://gitlab.gnome.org/GNOME/libxslt/-/raw/master/Copyright" },
  zlib: { url: "https://raw.githubusercontent.com/madler/zlib/master/LICENSE" },
  lz4: { url: "https://raw.githubusercontent.com/lz4/lz4/dev/lib/LICENSE" },
  zstd: { url: "https://raw.githubusercontent.com/facebook/zstd/dev/LICENSE" },
  winpthreads: { url: "https://raw.githubusercontent.com/mingw-w64/mingw-w64/master/mingw-w64-libraries/winpthreads/COPYING" },
  kerberos: { url: "https://raw.githubusercontent.com/krb5/krb5/master/NOTICE" },
};

const KNOWN = [
  { match: /^(libcrypto|libssl)-\d+(-x64)?\.dll$/i, project: "OpenSSL", licence: "Apache-2.0", source: "https://www.openssl.org/source/", text: "apache" },
  { match: /^libiconv-\d+\.dll$/i, project: "GNU libiconv", licence: "LGPL-2.1-or-later (used unmodified, as a separate library)", source: "https://ftp.gnu.org/pub/gnu/libiconv/", text: "lgpl" },
  { match: /^libintl-\d+\.dll$/i, project: "GNU gettext (libintl)", licence: "LGPL-2.1-or-later (used unmodified, as a separate library)", source: "https://ftp.gnu.org/pub/gnu/gettext/", text: "lgpl" },
  { match: /^libwinpthread-\d+\.dll$/i, project: "mingw-w64 winpthreads", licence: "MIT and BSD-3-Clause", source: "https://www.mingw-w64.org/", text: "winpthreads" },
  { match: /^libxml2(-\d+)?\.dll$/i, project: "libxml2", licence: "MIT", source: "https://gitlab.gnome.org/GNOME/libxml2", text: "libxml2" },
  { match: /^lib(e?xslt)(-\d+)?\.dll$/i, project: "libxslt", licence: "MIT", source: "https://gitlab.gnome.org/GNOME/libxslt", text: "libxslt" },
  { match: /^icu(dt|in|uc|io|tu)\d*\.dll$/i, project: "ICU", licence: "Unicode-3.0", source: "https://github.com/unicode-org/icu", text: "icu" },
  { match: /^liblz4\.dll$/i, project: "LZ4", licence: "BSD-2-Clause", source: "https://github.com/lz4/lz4", text: "lz4" },
  { match: /^libzstd\.dll$/i, project: "Zstandard", licence: "BSD-3-Clause", source: "https://github.com/facebook/zstd", text: "zstd" },
  { match: /^zlib1?\.dll$/i, project: "zlib", licence: "Zlib", source: "https://zlib.net/", text: "zlib" },
  { match: /^(krb5|comerr|gssapi|k5sprt|xpprof|krbcc)\d*\.dll$/i, project: "MIT Kerberos", licence: "MIT", source: "https://web.mit.edu/kerberos/", text: "kerberos" },
  { match: /^(vcruntime140(_1)?|msvcp140(_\d+|_atomic_wait|_codecvt_ids)?|concrt140)\.dll$/i, project: "Microsoft Visual C++ runtime", licence: "redistributed as allowed by the Microsoft Visual C++ redistribution terms", source: "https://learn.microsoft.com/cpp/windows/redistributing-visual-cpp-files" },
  { match: /^(libpq|libecpg|libecpg_compat|libpgtypes)\.dll$/i, project: "PostgreSQL", licence: "PostgreSQL License (above)" },
];

const libraries = readdirSync(join(postgres, "bin")).filter((f) => /\.dll$/i.test(f)).sort();
const unknown = libraries.filter((f) => !KNOWN.some((k) => k.match.test(f)));
if (unknown.length) {
  console.error(`unknown libraries – check their licences and add them to scripts/postgres-notices.mjs: ${unknown.join(", ")}`);
  process.exit(1);
}

const used = new Map();
for (const file of libraries) {
  const k = KNOWN.find((k) => k.match.test(file));
  const entry = used.get(k.project) ?? { ...k, files: [] };
  entry.files.push(file);
  used.set(k.project, entry);
}

async function text(key) {
  const t = TEXTS[key];
  if (t.file) return readFileSync(t.file, "utf8").trim();
  const response = await fetch(t.url);
  if (!response.ok) throw new Error(`${t.url}: ${response.status}`);
  return (await response.text()).trim();
}

const parts = [];
for (const entry of used.values()) {
  parts.push(
    "",
    "=".repeat(78),
    `${entry.project}: ${entry.files.join(", ")}`,
    `Licence: ${entry.licence}`,
    ...(entry.source ? [`Source: ${entry.source}`] : []),
    "=".repeat(78),
  );
  if (entry.text) parts.push("", await text(entry.text));
}
appendFileSync(notices, `${parts.join("\n")}\n`);
console.log(`postgresql notices: ${libraries.length} libraries of ${used.size} projects`);
