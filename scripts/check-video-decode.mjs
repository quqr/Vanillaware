#!/usr/bin/env node
/**
 * 影片解码链路自检：不需要浏览器，Node 里跑 `src/lib/videoDecode.ts` 的真实代码。
 *
 *   npm run check:video -- "<x.usm>" [gop]
 *
 * 步骤：
 * 1. 调 `usm_gop_probe`（Rust 例子）取某个 GOP 的字节——与 `usm_video_gop` 命令同一路径；
 * 2. 装一层**按 Chrome 对 AVC 的校验规则**做检查的 WebCodecs 模拟层：
 *    `configure` 必须带合法 avcC 且 profile/level 与 codec 串一致、
 *    每个 chunk 的 AVCC 长度前缀必须正好铺满缓冲区、`type:"key"` 必须含 IDR、
 *    chunk 里不能残留 AUD；
 * 3. 跑 `decodeGop`，核对送包数 / 出帧数 / 帧是否都被关闭；
 * 4. 有 ffmpeg 就把转换结果拼回 Annex-B 独立解码，核对帧数是否一致。
 *
 * 它验证的是**接线与码流结构**，不代表目标 WebView 的硬解能力。
 */
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const FFMPEG_CANDIDATES = [
  process.env.VW_FFMPEG,
  "C:\\Program Files\\Krita (x64)\\bin\\ffmpeg.exe",
  "ffmpeg",
].filter(Boolean);

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");

function usage() {
  console.error('用法：npm run check:video -- "<x.usm>" [gop]');
  process.exit(2);
}

const [usm, gopArg] = process.argv.slice(2);
if (!usm) usage();
const gop = Number(gopArg ?? 0);

const dir = mkdtempSync(join(tmpdir(), "vw-gop-"));
const dumpPath = join(dir, "gop.h264");

// ─── 1. 后端取字节 ─────────────────────────────────────────────

const probe = spawnSync(
  "cargo",
  [
    "run",
    "-q",
    "-p",
    "vw-media",
    "--example",
    "usm_gop_probe",
    "--",
    usm,
    String(gop),
    dumpPath,
  ],
  { cwd: ROOT, encoding: "utf8", maxBuffer: 1 << 26 },
);
if (probe.status !== 0) {
  console.error(probe.stderr || probe.stdout);
  process.exit(probe.status ?? 1);
}
const info = JSON.parse(probe.stdout.trim().split("\n").pop());
const stream = new Uint8Array(readFileSync(dumpPath));
console.log(
  `索引：${info.width}×${info.height} ${info.codec} ${info.fps}fps / ${info.frames} 帧 / ` +
    `${info.gopCount} 个关键帧 / 音频 ${info.audio}`,
);
console.log(`GOP ${gop}：帧 ${info.startFrame}..${info.startFrame + info.gopFrames} / ${stream.length} 字节`);

const { decodeGop, splitFrames, annexBToAvcc, buildAvcC } = await import(
  new URL("../src/lib/videoDecode.ts", import.meta.url).href
);

// ─── 2. 严格校验的 WebCodecs 模拟层 ────────────────────────────

function parseAvcC(bytes) {
  if (bytes[0] !== 1) throw new TypeError(`avcC configurationVersion=${bytes[0]}（应为 1）`);
  const lengthSize = (bytes[4] & 0x03) + 1;
  const spsCount = bytes[5] & 0x1f;
  if (spsCount < 1) throw new TypeError("avcC 里没有 SPS");
  let at = 6;
  const spsList = [];
  for (let i = 0; i < spsCount; i += 1) {
    const len = (bytes[at] << 8) | bytes[at + 1];
    spsList.push(bytes.subarray(at + 2, at + 2 + len));
    at += 2 + len;
  }
  const ppsCount = bytes[at];
  at += 1;
  const ppsList = [];
  for (let i = 0; i < ppsCount; i += 1) {
    const len = (bytes[at] << 8) | bytes[at + 1];
    ppsList.push(bytes.subarray(at + 2, at + 2 + len));
    at += 2 + len;
  }
  if (at !== bytes.length) throw new TypeError(`avcC 尾部多出 ${bytes.length - at} 字节`);
  if (!ppsList.length) throw new TypeError("avcC 里没有 PPS");
  return { lengthSize, sps: spsList[0], pps: ppsList[0] };
}

function parseAvcc(data, lengthSize) {
  const nals = [];
  let at = 0;
  while (at < data.length) {
    if (at + lengthSize > data.length) throw new TypeError(`AVCC 长度字段越界 @${at}`);
    let len = 0;
    for (let i = 0; i < lengthSize; i += 1) len = (len << 8) | data[at + i];
    at += lengthSize;
    if (len <= 0 || at + len > data.length) {
      throw new TypeError(`AVCC 长度 ${len} 越界（剩余 ${data.length - at}）`);
    }
    nals.push(data.subarray(at, at + len));
    at += len;
  }
  return nals;
}

/** Annex-B 模式下的 NAL 切分（严格：必须至少一个起始码）。 */
function splitAnnexB(data) {
  const nals = [];
  let at = -1;
  for (let i = 0; i + 3 <= data.length; i += 1) {
    if (data[i] !== 0 || data[i + 1] !== 0) continue;
    const header = data[i + 2] === 1 ? i + 3 : data[i + 2] === 0 && data[i + 3] === 1 ? i + 4 : -1;
    if (header < 0) continue;
    if (at >= 0) nals.push(data.subarray(at, i));
    at = header;
    i = header - 1;
  }
  if (at < 0) throw new TypeError("Annex-B chunk 里没有起始码");
  nals.push(data.subarray(at));
  return nals.filter((nal) => nal.length);
}

const stats = { configured: null, decoded: 0, keyChunks: 0, nalTypes: [] };

class MockVideoDecoder {
  constructor(init) {
    this._output = init.output;
    this._error = init.error;
    this.state = "unconfigured";
  }

  static async isConfigSupported(config) {
    return { supported: true, config };
  }

  configure(config) {
    if (this.state === "closed") throw new TypeError("decoder 已关闭");
    if (!/^avc1\.[0-9a-f]{6}$/i.test(config.codec ?? "")) {
      throw new TypeError(`codec 串不合法：${config.codec}`);
    }
    if (!config.description) {
      // 无 description 时 AVC 走 Annex-B；这条路径由 decode() 直接按起始码切。
      stats.configured = { codec: config.codec, lengthSize: 0 };
      this._annexB = true;
      this.state = "configured";
      return;
    }
    const avcC = parseAvcC(config.description);
    const profile = Number.parseInt(config.codec.slice(5, 7), 16);
    const level = Number.parseInt(config.codec.slice(9, 11), 16);
    if (avcC.sps[1] !== profile) {
      throw new TypeError(`codec profile ${profile} != SPS ${avcC.sps[1]}`);
    }
    if (avcC.sps[3] !== level) throw new TypeError(`codec level ${level} != SPS ${avcC.sps[3]}`);
    stats.configured = { codec: config.codec, lengthSize: avcC.lengthSize };
    this._avcC = avcC;
    this.state = "configured";
  }

  decode(chunk) {
    if (this.state !== "configured") {
      this._error(new Error("decoder 尚未 configure"));
      return;
    }
    let nals;
    try {
      nals = this._annexB ? splitAnnexB(chunk.data) : parseAvcc(chunk.data, this._avcC.lengthSize);
    } catch (error) {
      this._error(error);
      return;
    }
    const types = nals.map((nal) => nal[0] & 0x1f);
    stats.nalTypes.push(types);
    if (types.includes(9)) {
      this._error(new Error("chunk 里残留 AUD（type 9）"));
      return;
    }
    if (chunk.type === "key") {
      stats.keyChunks += 1;
      if (!types.includes(5)) {
        this._error(new Error(`key chunk 里没有 IDR：${types.join(",")}`));
        return;
      }
    }
    stats.decoded += 1;
    this._output({
      width: info.width,
      height: info.height,
      timestamp: chunk.timestamp,
      closed: false,
      close() {
        this.closed = true;
      },
    });
  }

  async flush() {
    if (this.state === "closed") throw new Error("flush 时 decoder 已关闭");
  }

  close() {
    this.state = "closed";
  }
}

globalThis.VideoDecoder = MockVideoDecoder;
globalThis.EncodedVideoChunk = class {
  constructor(init) {
    this.type = init.type;
    this.timestamp = init.timestamp;
    this.data = init.data;
    this.byteLength = init.data.byteLength;
  }
};

// ─── 3. 跑真实模块 ─────────────────────────────────────────────

const problems = [];
const packets = splitFrames(stream);
if (packets.length !== info.gopFrames) {
  problems.push(`splitFrames 得到 ${packets.length} 帧（索引说 ${info.gopFrames}）`);
}
if (!packets[0]?.key) problems.push("第一帧没有被标成关键帧");

const received = [];
let result = null;
try {
  result = await decodeGop(stream, info, info.startFrame, (frame) => {
    if (frame.closed) problems.push("转换时帧已被关闭");
    // 前端按 timestamp 反推绝对帧号（用作缓存 key），这里确认能推对。
    const index = Math.round((frame.timestamp * info.fps) / 1_000_000);
    const want = info.startFrame + received.length;
    if (index !== want) problems.push(`帧号推导 ${index} != ${want}`);
    received.push(frame);
  });
} catch (error) {
  problems.push(`decodeGop 抛错：${error?.message ?? error}`);
}

if (result) {
  if (result.chunks !== packets.length) {
    problems.push(`送包 ${result.chunks} != 切帧 ${packets.length}`);
  }
  if (result.decoded !== result.chunks) {
    problems.push(`出帧 ${result.decoded} != 送包 ${result.chunks}`);
  }
}
if (received.length !== packets.length) {
  problems.push(`onFrame 收到 ${received.length} 帧（应为 ${packets.length}）`);
}
if (received.some((frame) => !frame.closed)) problems.push("有帧没被关闭（泄漏）");

// ─── 4. ffmpeg 独立解码（有就跑）────────────────────────────────

const ffmpeg = FFMPEG_CANDIDATES.find((path) => path === "ffmpeg" || existsSync(path));
let ffmpegFrames = null;
if (ffmpeg) {
  const avcC = parseAvcC(buildAvcC(Uint8Array.from(Buffer.from(info.sps, "hex")), Uint8Array.from(Buffer.from(info.pps, "hex"))));
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
  const warnings = String(raw.stderr ?? "").split("\n").filter((line) => line.trim());
  if (warnings.length) problems.push(`ffmpeg 告警 ${warnings.length} 条：${warnings[0]}`);
  if (raw.status === 0) {
    ffmpegFrames = raw.stdout.length / (info.width * info.height);
    if (Math.round(ffmpegFrames) !== packets.length) {
      problems.push(`ffmpeg 回解 ${ffmpegFrames} 帧（应为 ${packets.length}）`);
    }
  } else {
    problems.push("ffmpeg 回解失败");
  }
}

console.log(
  `解码：codec=${stats.configured?.codec} 路径=${result?.strategy ?? "?"} ` +
    `送包 ${result?.chunks ?? 0} / 出帧 ${result?.decoded ?? 0} / 关键 chunk ${stats.keyChunks}`,
);
if (result?.warning) problems.push(`出现 warning：${result.warning}`);
if (result && result.strategy !== "avcC") problems.push(`走的是备用路径 ${result.strategy}`);
console.log(
  `首个 chunk 的 NAL 类型：${(stats.nalTypes[0] ?? []).join(",")}` +
    (ffmpegFrames === null ? "（未找到 ffmpeg，跳过独立解码）" : `；ffmpeg 独立解码 ${Math.round(ffmpegFrames)} 帧`),
);

// ─── 回归：攒帧会耗光硬解输出池 ─────────────────────────────
// 旧实现把 VideoFrame 攒进数组、等解码完再统一转位图。硬解的输出缓冲只有十几块，
// 攒满就再也出不来帧（线上表现：只解出 13/30 帧，flush 一直不返回）。
// 这里用一个「池子 13 块、不还就停」的假解码器，确认现在的实现当场归还。
{
  const POOL = 13;
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
        ? Promise.reject(new Error(`输出缓冲耗尽（${POOL} 块都没还）`))
        : Promise.resolve();
    }
    close() {
      this.state = "closed";
    }
  };
  let got = 0;
  try {
    const result = await decodeGop(stream, info, info.startFrame, () => {
      got += 1;
    });
    if (got !== packets.length) problems.push(`池子回归：只解出 ${got}/${packets.length} 帧`);
    if (result.warning) problems.push(`池子回归：${result.warning}`);
  } catch (error) {
    problems.push(`池子回归：${error?.message ?? error}`);
  }
  globalThis.VideoDecoder = Real;
  console.log(`输出池回归：${got}/${packets.length} 帧（池子 ${POOL} 块）`);
}

// ─── 窗口逻辑：什么时候取下一段（纯函数）──────────────────────
// 以前是「播到段末才取」，取流 + 解码那 100~200 ms 里没有画面 → 每跨一个 GOP 闪一下。
{
  const { gopIndexAt, gopRange, nextGopToLoad } = await import(
    new URL("../src/lib/videoWindow.ts", import.meta.url).href
  );
  const keyframes = [0, 30, 60, 90];
  const total = 120;
  const cache = new Set();
  for (let frame = 0; frame < 30; frame += 1) cache.add(frame);
  const has = (frame) => cache.has(frame);
  const ask = (current, loadedRange) =>
    nextGopToLoad({
      keyframes,
      total,
      current,
      loaded: loadedRange,
      has,
      prefetchAhead: 12,
    });

  const gop0 = gopRange(keyframes, total, 0);
  const gop1 = gopRange(keyframes, total, 1);
  const cases = [
    ["段内不重复取", ask(0, gop0), null],
    ["还没到预取线", ask(17, gop0), null],
    ["距段末 12 帧预取下一段", ask(18, gop0), 1],
    ["下一段已在手不重复取", ask(20, gop1), null],
    ["拖动跳段取目标段", ask(100, gop1), 3],
    ["最后一段不再取", ask(119, gopRange(keyframes, total, 3)), null],
    ["帧号越界取最后一段", gopIndexAt(keyframes, 9999), 3],
  ];
  for (const [label, actual, expected] of cases) {
    if (actual !== expected) problems.push(`窗口逻辑「${label}」：${actual} != ${expected}`);
  }
  // 补齐 30..59 后，从段 0 的末尾看到的应该是「下一段已在手」
  for (let frame = 30; frame < 60; frame += 1) cache.add(frame);
  if (ask(25, gop0) !== null) problems.push("窗口逻辑：下一段已在缓存里时仍在重复取");
  console.log("窗口逻辑：播放跨段不再出现空档（7 项断言）");
}

// ─── 位图缓存：解出 30 帧却缓存 0 帧（线上事故复现）──────────────
// 事故现场：底栏写着「送包 30 · 解出 30 · 缓存 0 帧」，画面全黑。
// 原因不在解码器，而在合并写进了 `setFrames` 的 updater —— updater 是**延后执行**的，
// 调用方在 finally 里已经把源数组清空，updater 拿到的是空数组。
{
  const { mergeFrameBatch } = await import(
    new URL("../src/lib/frameCache.ts", import.meta.url).href
  );
  const fake = () => ({ closed: false, close() { this.closed = true; } });
  const range = (from, count) =>
    Array.from({ length: count }, (_, i) => ({ index: from + i, bitmap: fake() }));

  // 1) 首段全部落表
  let merge = mergeFrameBatch(new Map(), range(0, 30), 0);
  if (merge.next.size !== 30) problems.push(`缓存「首段」：${merge.next.size} != 30`);
  if (merge.dropped.length) problems.push(`缓存「首段」：不该淘汰任何帧`);

  // 2) 预取下一段：上一段仍在（跨 GOP 才有画面接续）
  merge = mergeFrameBatch(merge.next, range(30, 30), 0);
  if (merge.next.size !== 60) problems.push(`缓存「预取下一段」：${merge.next.size} != 60`);
  if (merge.dropped.length) problems.push(`缓存「预取下一段」：不该淘汰任何帧`);

  // 3) 播到第三段：最老的一段（0..29）淘汰，避免内存随时长涨
  merge = mergeFrameBatch(merge.next, range(60, 30), 30);
  if (merge.next.size !== 60) problems.push(`缓存「第三段」：${merge.next.size} != 60`);
  if (merge.dropped.length !== 30) problems.push(`缓存「第三段」：淘汰 ${merge.dropped.length} != 30`);
  if (merge.next.has(0)) problems.push("缓存「第三段」：最老一段没被淘汰");
  if (!merge.next.has(89)) problems.push("缓存「第三段」：最新一段没进表");

  // 4) 同一帧被新位图覆盖：旧的进 dropped，且不改入参
  const stale = fake();
  const previous = new Map([[30, stale]]);
  const replacement = { index: 30, bitmap: fake() };
  const replaced = mergeFrameBatch(previous, [replacement], 0);
  if (replaced.next.get(30) !== replacement.bitmap) problems.push("缓存「覆盖同帧」：没换成新位图");
  if (!replaced.dropped.includes(stale)) problems.push("缓存「覆盖同帧」：旧位图没交出去关");
  if (previous.get(30) !== stale) problems.push("缓存：入参被就地修改（必须纯）");

  // 5) 事故复现：延后执行的 updater 读到被清空的可变数组
  let mutable = range(0, 30);
  const snapshot = mutable;
  const lazyUpdater = () => mergeFrameBatch(new Map(), mutable, 0);
  mutable = [];
  if (lazyUpdater().next.size !== 0) problems.push("缓存：事故复现前提失效（updater 应读到空数组）");
  if (mergeFrameBatch(new Map(), snapshot, 0).next.size !== 30) {
    problems.push("缓存：不可变快照仍然丢帧");
  }
  console.log("位图缓存：解出 30 帧必须落表 30 张（含「延后 updater 读空数组」事故复现）");
}

console.log(problems.length ? `\nFAIL\n${problems.join("\n")}` : "\nPASS 全部通过");
process.exit(problems.length ? 1 : 0);
