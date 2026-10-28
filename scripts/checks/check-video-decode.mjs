import { spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { Report } from "../support/assert.mjs";
import { findFfmpeg, readGop } from "../support/usm.mjs";
import { installVideoDecoder, parseAvcC, parseAvcc } from "../support/webcodecs.mjs";

const [usm, gopArg] = process.argv.slice(2);
if (!usm) {
  console.error('usage: npm run check:video -- "<file.usm>" [gop]');
  process.exit(2);
}
const gopIndex = Number(gopArg ?? 0);

const dir = mkdtempSync(join(tmpdir(), "vanillaware-gop-"));
const dumpPath = join(dir, "gop.h264");

const { info, stream } = readGop({ usm, gop: gopIndex, dumpPath });
console.log(
  `index: ${info.width}x${info.height} ${info.codec} ${info.fps}fps / ${info.frames} frames / ` +
    `${info.gopCount} keyframes / audio ${info.audio}`,
);
console.log(
  `gop ${gopIndex}: frames ${info.startFrame}..${info.startFrame + info.gopFrames} / ${stream.length} bytes`,
);

const { decodeGop, splitFrames, annexBToAvcc, buildAvcC } = await import(
  new URL("../../src/lib/videoDecode.ts", import.meta.url).href
);

const stats = { configured: null, decoded: 0, keyChunks: 0, nalTypes: [] };
installVideoDecoder({ width: info.width, height: info.height, stats });

const report = new Report();
const packets = splitFrames(stream);
report.equal(packets.length, info.gopFrames, "splitFrames count");
report.check(packets[0]?.key, "first frame is not marked as a keyframe");

const received = [];
let result = null;
try {
  result = await decodeGop(stream, info, info.startFrame, (frame) => {
    if (frame.closed) report.add("frame was already closed during conversion");
    const index = Math.round((frame.timestamp * info.fps) / 1_000_000);
    report.equal(index, info.startFrame + received.length, "frame index derived from timestamp");
    received.push(frame);
  });
} catch (error) {
  report.add(`decodeGop threw: ${error?.message ?? error}`);
}

if (result) {
  report.equal(result.chunks, packets.length, "chunks fed");
  report.equal(result.decoded, result.chunks, "frames out");
  report.equal(result.strategy, "avcC", "decode path");
  if (result.warning) report.add(`warning: ${result.warning}`);
}
report.equal(received.length, packets.length, "onFrame calls");
report.check(
  !received.some((frame) => !frame.closed),
  "a frame was never closed (leak)",
);

const ffmpeg = findFfmpeg();
let ffmpegFrames = null;
if (ffmpeg) {
  const avcC = parseAvcC(
    buildAvcC(
      Uint8Array.from(Buffer.from(info.sps, "hex")),
      Uint8Array.from(Buffer.from(info.pps, "hex")),
    ),
  );
  const parts = [];
  const push = (nal) => {
    parts.push(Buffer.from([0, 0, 0, 1]), Buffer.from(nal));
  };
  push(avcC.sps);
  push(avcC.pps);
  for (const packet of packets) {
    for (const nal of parseAvcc(annexBToAvcc(packet.data), 4)) push(nal);
  }
  const annexB = join(dir, "roundtrip.h264");
  writeFileSync(annexB, Buffer.concat(parts));
  const raw = spawnSync(
    ffmpeg,
    ["-hide_banner", "-v", "warning", "-f", "h264", "-i", annexB, "-f", "rawvideo", "-pix_fmt", "gray", "-"],
    { maxBuffer: 1 << 29 },
  );
  const warnings = String(raw.stderr ?? "")
    .split("\n")
    .filter((line) => line.trim());
  if (warnings.length) report.add(`ffmpeg reported ${warnings.length}: ${warnings[0]}`);
  if (raw.status === 0) {
    ffmpegFrames = raw.stdout.length / (info.width * info.height);
    report.equal(Math.round(ffmpegFrames), packets.length, "ffmpeg frames");
  } else {
    report.add("ffmpeg failed to decode the stream");
  }
}

console.log(
  `decode: codec=${stats.configured?.codec} path=${result?.strategy ?? "?"} ` +
    `chunks ${result?.chunks ?? 0} / frames ${result?.decoded ?? 0} / key chunks ${stats.keyChunks}`,
);
console.log(
  `first chunk NAL types: ${(stats.nalTypes[0] ?? []).join(",")}` +
    (ffmpegFrames === null
      ? " (ffmpeg not found, independent decode skipped)"
      : `; ffmpeg decoded ${Math.round(ffmpegFrames)} frames`),
);

const POOL = 13;
{
  let open = 0;
  const Real = globalThis.VideoDecoder;
  globalThis.VideoDecoder = class {
    constructor(init) {
      this._output = init.output;
      this.state = "unconfigured";
      this.decodeQueueSize = 0;
    }
    static async isConfigSupported() {
      return { supported: true };
    }
    configure() {
      this.state = "configured";
      this._stalled = false;
    }
    decode() {
      if (this._stalled) return;
      const frame = {
        closed: false,
        close() {
          if (!this.closed) {
            this.closed = true;
            open -= 1;
          }
        },
      };
      open += 1;
      this._output(frame);
      if (open >= POOL) this._stalled = true;
    }
    flush() {
      return this._stalled
        ? Promise.reject(new Error(`output pool exhausted (${POOL} frames held)`))
        : Promise.resolve();
    }
    close() {
      this.state = "closed";
    }
  };
  let got = 0;
  try {
    const pooled = await decodeGop(stream, info, info.startFrame, () => {
      got += 1;
    });
    report.equal(got, packets.length, "frames released with a small output pool");
    if (pooled.warning) report.add(`output pool regression: ${pooled.warning}`);
  } catch (error) {
    report.add(`output pool regression: ${error?.message ?? error}`);
  }
  globalThis.VideoDecoder = Real;
  console.log(`output pool: ${got}/${packets.length} frames with a ${POOL}-slot pool`);
}

process.exit(report.print());
