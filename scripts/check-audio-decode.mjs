#!/usr/bin/env node
/**
 * lopus 解码链路自检：不需要浏览器，直接在 Node 里跑 `src/lib/audioDecode.ts`。
 *
 *   node --experimental-strip-types scripts/check-audio-decode.mjs <x.awb>
 *
 * 它做两件事：
 *
 * 1. **按 AFS2 结构抽真实 lopus 包**（每种「声道数 × 步长」形状各取一条轨道）；
 * 2. **装一层按 Chrome 行为严格校验的 WebCodecs 模拟层**，把包喂进真实的 `decodeClip`，
 *    检查帧数、声道数、pre-skip、以及每个样本的落位。
 *
 * 模拟层不是宽松替身：`AudioData.copyTo` 必须收到 2 个参数（`planeIndex` 是 required member）、
 * 目标数组类型必须与 `format` 匹配、长度必须够。所以「漏传第二个参数」这类错误
 * 会在这里抛出与浏览器一致的 `TypeError`。
 *
 * 它验证的是**接线与布局**（包切分、plane 拷贝、时间戳落位、pre-skip 裁剪），
 * 不解 Opus 码流本身——码流真解码由 ffmpeg 交叉验证，见 docs/audio-lopus.md。
 */
import { readFileSync } from "node:fs";
import { basename } from "node:path";

const FRAMES_PER_PACKET = 960;
const PACKET_MICROS = 20_000;
const FORMATS = ["f32-planar", "f32", "s16"];

const path = process.argv[2];
if (!path) {
  console.error("用法：node --experimental-strip-types scripts/check-audio-decode.mjs <x.awb>");
  process.exit(2);
}

// ─── AFS2 / lopus 抽取 ─────────────────────────────

const CHUNK_BASIC = 0x80000001;
const CHUNK_DATA = 0x80000004;

function lopusTracks(buf) {
  if (buf.toString("latin1", 0, 4) !== "AFS2") throw new Error("不是 AFS2 容器");
  const offSize = buf[5];
  const idSize = buf[6];
  const count = buf.readUInt32LE(8);
  const align = buf.readUInt16LE(0x0c) || 0x20;
  const offsAt = 0x10 + count * idSize;
  const readOffset = (i) =>
    offSize === 2 ? buf.readUInt16LE(offsAt + i * 2) : buf.readUInt32LE(offsAt + i * 4);
  const alignUp = (v) => Math.ceil(v / align) * align;

  const tracks = [];
  for (let i = 0; i < count; i += 1) {
    const from = alignUp(readOffset(i));
    const to = alignUp(readOffset(i + 1));
    if (to <= from || to > buf.length) continue;
    const blob = buf.subarray(from, to);
    let base = -1;
    for (let at = 0; at + 4 <= Math.min(blob.length, 0x20); at += 1) {
      if (blob.readUInt32LE(at) === CHUNK_BASIC) {
        base = at;
        break;
      }
    }
    if (base < 0) continue;
    const channels = blob[base + 0x09];
    const frameSize = blob.readUInt16LE(base + 0x0a);
    const sampleRate = blob.readUInt32LE(base + 0x0c);
    const dataOffset = blob.readUInt32LE(base + 0x10);
    const skip = blob.readUInt16LE(base + 0x1c);
    const dataAt = base + dataOffset;
    if (dataAt + 8 > blob.length || blob.readUInt32LE(dataAt) !== CHUNK_DATA) continue;

    const size = blob.readUInt32LE(dataAt + 4);
    const bodyStart = dataAt + 8;
    const stride = frameSize - 8;
    const total = Math.floor(Math.min(size, blob.length - bodyStart) / frameSize);
    if (stride <= 0 || total === 0) continue;
    // 只记位置，包体等到真正选中这条轨道时再拷（语料里有 400 MB 的 voice_jp.awb）。
    tracks.push({
      index: i,
      channels,
      stride,
      skip,
      sampleRate,
      total,
      frameSize,
      base: from + bodyStart,
    });
  }
  return tracks;
}

// ─── 严格 WebCodecs 模拟层 ─────────────────────────

const ARITY = "Failed to execute 'copyTo' on 'AudioData': 2 arguments required, but only 1 present.";
const NO_PLANE =
  "Failed to execute 'copyTo' on 'AudioData': Failed to read the 'planeIndex' property from 'AudioDataCopyToOptions': Required member is undefined.";

const unit = (globalIndex, channel) =>
  Math.fround((((globalIndex * 3 + channel * 7) % 1024) / 1024) - 0.5);

const CODECS = {
  "f32-planar": { kind: "f32", store: unit, restore: (v) => v },
  f32: { kind: "f32", store: unit, restore: (v) => v },
  s16: {
    kind: "s16",
    store: (g, ch) => Math.round(unit(g, ch) * 32768),
    restore: (v) => v / 32768,
  },
};

let stride = 0;
const stats = { decoded: 0 };

class MockAudioData {
  constructor({ format, numberOfFrames, numberOfChannels, timestamp, base }) {
    this.format = format;
    this.numberOfFrames = numberOfFrames;
    this.numberOfChannels = numberOfChannels;
    this.timestamp = timestamp;
    this.sampleRate = 48000;
    this._base = base;
  }

  copyTo(destination, options) {
    if (arguments.length < 2) throw new TypeError(ARITY);
    if (options === null || typeof options !== "object") {
      throw new TypeError("AudioDataCopyToOptions 必须是对象");
    }
    const planeIndex = options.planeIndex;
    if (planeIndex === undefined) throw new TypeError(NO_PLANE);
    const planar = this.format.endsWith("-planar");
    const planes = planar ? this.numberOfChannels : 1;
    if (planeIndex < 0 || planeIndex >= planes) {
      throw new RangeError(`planeIndex ${planeIndex} 超出 plane 数 ${planes}`);
    }
    const offset = options.frameOffset ?? 0;
    const count = options.frameCount ?? this.numberOfFrames - offset;
    const need = planar ? count : count * this.numberOfChannels;
    const spec = CODECS[this.format];
    const Ctor = { u8: Uint8Array, s16: Int16Array, s32: Int32Array, f32: Float32Array }[spec.kind];
    if (!(destination instanceof Ctor)) {
      throw new TypeError(`目标数组必须是 ${Ctor.name}`);
    }
    if (destination.length < need) {
      throw new TypeError(`目标数组太短：需要 ${need}，实际 ${destination.length}`);
    }
    const lanes = planar ? 1 : this.numberOfChannels;
    for (let i = 0; i < count; i += 1) {
      for (let lane = 0; lane < lanes; lane += 1) {
        const channel = planar ? planeIndex : lane;
        destination[i * lanes + lane] = spec.store(this._base + offset + i, channel);
      }
    }
  }

  close() {}
}

function installWebCodecs(format) {
  stats.decoded = 0;
  globalThis.AudioDecoder = class {
    constructor(init) {
      this._output = init.output;
      this.state = "unconfigured";
    }
    configure(config) {
      if (config.codec !== "opus") throw new TypeError(`不支持的编码 ${config.codec}`);
      if (!(config.sampleRate > 0) || !(config.numberOfChannels > 0)) {
        throw new TypeError("configure 缺少 sampleRate / numberOfChannels");
      }
      this.config = config;
      this.state = "configured";
    }
    decode(chunk) {
      if (this.state !== "configured") throw new Error("解码器尚未 configure");
      if (chunk.byteLength !== stride) {
        throw new Error(`包长 ${chunk.byteLength} 与步长 ${stride} 不符`);
      }
      stats.decoded += 1;
      this._output(
        new MockAudioData({
          format,
          numberOfFrames: FRAMES_PER_PACKET,
          numberOfChannels: this.config.numberOfChannels,
          timestamp: chunk.timestamp,
          base: Math.round(chunk.timestamp / PACKET_MICROS) * FRAMES_PER_PACKET,
        }),
      );
    }
    async flush() {}
    close() {
      this.state = "closed";
    }
  };
  globalThis.EncodedAudioChunk = class {
    constructor(init) {
      this.type = init.type;
      this.timestamp = init.timestamp;
      this.data = init.data;
      this.byteLength = init.data.byteLength;
    }
  };
}

// ─── 校验 ──────────────────────────────────────────

/** 按帧头 8 字节切出裸 Opus 包。 */
function materialize(buf, track) {
  const packets = Buffer.allocUnsafe(track.total * track.stride);
  for (let k = 0; k < track.total; k += 1) {
    const at = track.base + k * track.frameSize;
    buf.copy(packets, k * track.stride, at, at + track.stride);
  }
  return packets;
}

const { decodeClip } = await import("../src/lib/audioDecode.ts");

function probes(frames) {
  const picks = [0, 1, 119, 120, 959, 960, Math.floor(frames / 2), frames - 1];
  return [...new Set(picks.filter((g) => g >= 0 && g < frames))].sort((a, b) => a - b);
}

async function check(buf, track, format) {
  stride = track.stride;
  const packets = materialize(buf, track);
  installWebCodecs(format);
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
  const spec = CODECS[format];
  const problems = [];

  let decoded;
  try {
    decoded = await decodeClip(clip, data);
  } catch (error) {
    return [`#${track.index} ${format}`, [`抛错：${error?.message ?? error}`]];
  }

  if (decoded.frames !== want) problems.push(`帧数 ${decoded.frames} != ${want}`);
  if (decoded.channels.length !== clip.channels) {
    problems.push(`声道 ${decoded.channels.length} != ${clip.channels}`);
  }
  if (decoded.skipped !== clip.skip) problems.push(`pre-skip ${decoded.skipped} != ${clip.skip}`);
  if (stats.decoded !== track.total) problems.push(`解码包 ${stats.decoded} != ${track.total}`);
  for (const g of probes(decoded.frames)) {
    const globalIndex = g + clip.skip;
    for (let ch = 0; ch < clip.channels; ch += 1) {
      const expected = spec.restore(spec.store(globalIndex, ch));
      if (Math.abs(decoded.channels[ch][g] - expected) > 1e-6) {
        problems.push(`样本[${g}]ch${ch} 落位错误`);
      }
    }
  }
  const detail = `${track.total} 包 / ${decoded.frames} 帧 / ${clip.channels}ch / ${decoded.durationSec.toFixed(3)}s`;
  return [`#${track.index} ${format}`, problems.length ? problems.slice(0, 3) : null, detail];
}

const buffer = readFileSync(path);
const tracks = lopusTracks(buffer);
if (tracks.length === 0) {
  console.error(`${basename(path)} 里没有 lopus 轨道`);
  process.exit(1);
}

// 每种「声道 × 步长」形状取一条，覆盖 200/240/320 这类不同包长。
const shapes = new Map();
for (const track of tracks) {
  const key = `${track.channels}x${track.stride}`;
  if (!shapes.has(key)) shapes.set(key, track);
}

console.log(`${basename(path)}：${tracks.length} 条 lopus 轨道，测 ${shapes.size} 种形状 × ${FORMATS.length} 种 format`);

let failed = 0;
for (const track of shapes.values()) {
  for (const format of FORMATS) {
    const [label, problems, detail] = await check(buffer, track, format);
    if (problems) {
      failed += 1;
      console.log(`FAIL  ${label.padEnd(16)} ${problems.join("; ")}`);
    } else {
      console.log(`PASS  ${label.padEnd(16)} ${detail}`);
    }
  }
}

console.log(`\n${failed === 0 ? "全部通过" : `${failed} 项失败`}`);
process.exit(failed === 0 ? 0 : 1);
