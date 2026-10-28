import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

export const WORKSPACE = new URL("../../src-tauri/", import.meta.url).pathname.replace(
  /^\/([A-Za-z]:)/,
  "$1",
);

const FFMPEG_CANDIDATES = [
  process.env.FFMPEG_BIN,
  "C:\\Program Files\\Krita (x64)\\bin\\ffmpeg.exe",
  "ffmpeg",
].filter(Boolean);

export function findFfmpeg() {
  return FFMPEG_CANDIDATES.find((path) => path === "ffmpeg" || existsSync(path)) ?? null;
}

export function readGop({ usm, gop, dumpPath }) {
  const probe = spawnSync(
    "cargo",
    [
      "run",
      "-q",
      "-p",
      "media",
      "--example",
      "usm_gop_probe",
      "--",
      usm,
      String(gop),
      dumpPath,
    ],
    { cwd: WORKSPACE, encoding: "utf8", maxBuffer: 1 << 26 },
  );
  if (probe.status !== 0) {
    throw new Error(probe.stderr || probe.stdout || "usm_gop_probe failed");
  }
  const info = JSON.parse(probe.stdout.trim().split("\n").pop());
  return { info, stream: new Uint8Array(readFileSync(dumpPath)) };
}
