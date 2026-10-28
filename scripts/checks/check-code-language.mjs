import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("../../", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");

const SCAN = ["src-tauri/crates", "src-tauri/src", "src", "spec/ksy", "scripts"];
const SKIP_DIRS = new Set(["node_modules", "target", "vendor", "gen", "dist", ".git"]);
const SKIP_FILES = new Set(["src/i18n/zh.ts"]);
const EXTENSIONS = new Set([".rs", ".ts", ".tsx", ".mjs", ".ksy", ".toml", ".json"]);
const CJK = /[\u3000-\u303f\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uff00-\uffef]/;

function walk(dir, out) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return;
  }
  for (const entry of entries) {
    const path = join(dir, entry);
    const info = statSync(path);
    if (info.isDirectory()) {
      if (!SKIP_DIRS.has(entry)) walk(path, out);
      continue;
    }
    const dot = entry.lastIndexOf(".");
    if (dot < 0 || !EXTENSIONS.has(entry.slice(dot))) continue;
    out.push(path);
  }
}

const files = [];
for (const dir of SCAN) walk(join(ROOT, dir), files);

const offenders = [];
for (const path of files) {
  const rel = relative(ROOT, path).replace(/\\/g, "/");
  if (SKIP_FILES.has(rel)) continue;
  const lines = readFileSync(path, "utf8").split("\n");
  lines.forEach((line, index) => {
    if (CJK.test(line)) offenders.push(`${rel}:${index + 1}: ${line.trim()}`);
  });
}

if (offenders.length) {
  console.error(`${offenders.length} line(s) contain CJK outside the allow list:`);
  for (const offender of offenders) console.error(`  ${offender}`);
  process.exit(1);
}
console.log(`scanned ${files.length} files, no CJK outside src/i18n/zh.ts`);
