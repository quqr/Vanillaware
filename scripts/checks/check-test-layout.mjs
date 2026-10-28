import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const ROOT = new URL("../../", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const CRATES = join(ROOT, "src-tauri/crates");

const problems = [];

function walk(dir, out) {
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return;
  }
  for (const entry of entries) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) walk(path, out);
    else if (entry.endsWith(".rs")) out.push(path);
  }
}

function rel(path) {
  return relative(ROOT, path).replace(/\\/g, "/");
}

for (const source of [CRATES, join(ROOT, "src-tauri/src")]) {
  const files = [];
  walk(source, files);
  for (const file of files) {
    if (readFileSync(file, "utf8").includes("cfg(test)")) {
      problems.push(`${rel(file)}: tests must live outside src/`);
    }
  }
}

for (const crate of readdirSync(CRATES)) {
  const examples = join(CRATES, crate, "examples");
  if (!existsSync(examples)) continue;
  for (const entry of readdirSync(examples)) {
    if (entry.endsWith(".rs")) problems.push(`${rel(join(examples, entry))}: examples/ must be empty`);
  }
}

for (const entry of readdirSync(join(ROOT, "scripts"))) {
  if (/^check-.*\.mjs$/.test(entry)) problems.push(`scripts/${entry}: move check scripts into scripts/checks/`);
}

for (const crate of readdirSync(CRATES)) {
  const tools = join(CRATES, crate, "tests/tools");
  if (!existsSync(tools)) continue;
  const manifest = readFileSync(join(CRATES, crate, "Cargo.toml"), "utf8");
  for (const entry of readdirSync(tools)) {
    if (!entry.endsWith(".rs")) continue;
    const name = entry.slice(0, -3);
    if (!manifest.includes(`name = "${name}"`)) {
      problems.push(`${rel(join(tools, entry))}: missing [[example]] entry in Cargo.toml`);
    }
  }
}

if (problems.length) {
  console.error("layout violations:");
  for (const problem of problems) console.error(`  ${problem}`);
  process.exit(1);
}
console.log("layout ok: no tests in src/, no examples/, checks under scripts/checks/");
