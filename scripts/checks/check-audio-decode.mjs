import { readFileSync } from "node:fs";
import { basename } from "node:path";

import { FRAMES_PER_PACKET, PACKET_MICROS, lopusTracks, materializeLopus } from "../support/afs2.mjs";
import { Report } from "../support/assert.mjs";
import { AUDIO_FORMATS, audioLayout, installAudioDecoder } from "../support/webcodecs.mjs";

const path = process.argv[2];
if (!path) {
  console.error("usage: npm run check:audio -- <file.awb>");
  process.exit(2);
}

const { decodeClip } = await import(
  new URL("../../src/lib/audioDecode.ts", import.meta.url).href
);

function probeIndices(frames) {
  const picks = [0, 1, 119, 120, 959, 960, Math.floor(frames / 2), frames - 1];
  return [...new Set(picks.filter((g) => g >= 0 && g < frames))].sort((a, b) => a - b);
}

async function check(buffer, track, format, report) {
  const packets = materializeLopus(buffer, track);
  const stats = { decoded: 0 };
  installAudioDecoder({
    format,
    framesPerPacket: FRAMES_PER_PACKET,
    packetMicros: PACKET_MICROS,
    stride: track.stride,
    stats,
  });

  const clip = {
    codec: "lopus",
    sampleRate: track.sampleRate,
    channels: track.channels,
    frames: track.total * FRAMES_PER_PACKET,
    durationMs: Math.round((track.total * FRAMES_PER_PACKET * 1000) / track.sampleRate),
    stride: track.stride,
    skip: track.skip,
    bytes: packets.length,
  };
  const data = packets.buffer.slice(packets.byteOffset, packets.byteOffset + packets.byteLength);
  const want = clip.frames - clip.skip;
  const spec = audioLayout(format);
  const local = [];

  let decoded;
  try {
    decoded = await decodeClip(clip, data);
  } catch (error) {
    return { label: `#${track.index} ${format}`, problems: [`threw: ${error?.message ?? error}`] };
  }

  if (decoded.frames !== want) local.push(`frames ${decoded.frames} != ${want}`);
  if (decoded.channels.length !== clip.channels) {
    local.push(`channels ${decoded.channels.length} != ${clip.channels}`);
  }
  if (decoded.skipped !== clip.skip) local.push(`pre-skip ${decoded.skipped} != ${clip.skip}`);
  if (stats.decoded !== track.total) local.push(`decoded packets ${stats.decoded} != ${track.total}`);
  for (const g of probeIndices(decoded.frames)) {
    const globalIndex = g + clip.skip;
    for (let ch = 0; ch < clip.channels; ch += 1) {
      const expected = spec.restore(spec.store(globalIndex, ch));
      if (Math.abs(decoded.channels[ch][g] - expected) > 1e-6) {
        local.push(`sample[${g}] ch${ch} landed wrong`);
      }
    }
  }

  const detail = `${track.total} packets / ${decoded.frames} frames / ${clip.channels}ch / ${decoded.durationSec.toFixed(3)}s`;
  return { label: `#${track.index} ${format}`, problems: local.length ? local.slice(0, 3) : null, detail };
}

const buffer = readFileSync(path);
const tracks = lopusTracks(buffer);
if (tracks.length === 0) {
  console.error(`${basename(path)} has no lopus track`);
  process.exit(1);
}

const shapes = new Map();
for (const track of tracks) {
  const key = `${track.channels}x${track.stride}`;
  if (!shapes.has(key)) shapes.set(key, track);
}

console.log(
  `${basename(path)}: ${tracks.length} lopus tracks, ${shapes.size} shapes x ${AUDIO_FORMATS.length} formats`,
);

const report = new Report();
for (const track of shapes.values()) {
  for (const format of AUDIO_FORMATS) {
    const { label, problems, detail } = await check(buffer, track, format, report);
    if (problems) {
      problems.forEach((problem) => report.add(`${label} ${problem}`));
    } else {
      console.log(`ok    ${label.padEnd(16)} ${detail}`);
    }
  }
}

process.exit(report.print());
