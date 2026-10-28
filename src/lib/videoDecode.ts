/**
 * USM 影片解码：Annex-B 分段 → AVCC → WebCodecs `VideoDecoder` → 逐帧位图。
 *
 * 后端按 GOP 给字节，这里拆帧、去掉 AUD、把起始码换成 4 字节长度，
 * 再用 avcC（由 SPS/PPS 拼）配置解码器。
 *
 * 几条硬约束，都是踩过的：
 *
 * 1. **输出帧要在 `output` 回调里当场消费、当场 `close()`**。
 *    解码器的输出缓冲是有配额的（硬解十几块）；攒着不还，池子一空解码器就停，
 *    表现就是「吐出 13/30 帧后 `flush()` 永不返回」。
 * 2. **每个 await 都要有超时**。平台解码器卡住时 `flush()` 可以永不 settle，
 *    界面会一直停在「解码中」，既没画面也没错误。
 * 3. **不能把「解出 0 帧」当成功**：0 帧只会让界面空着。
 * 4. **两条路径都试**：AVC 既能 `description` + AVCC，也能无 `description` 走 Annex-B；
 *    前者卡住或零输出时换后者再试。
 * 5. **每个阶段都要能指认**：配置 / 送包 / 出帧 / flush / 转画面分开报。
 */

import type { UsmVideo } from "./ipc";

export class UnsupportedVideoError extends Error {}

type AnyVideoDecoder = new (init: {
  output: (frame: unknown) => void;
  error: (error: unknown) => void;
}) => {
  configure: (config: Record<string, unknown>) => void;
  decode: (chunk: unknown) => void;
  flush: () => Promise<void>;
  close: () => void;
  state: string;
  decodeQueueSize?: number;
};

type DecoderCtor = AnyVideoDecoder & {
  isConfigSupported?: (config: Record<string, unknown>) => Promise<{ supported?: boolean }>;
};

function decoderCtor(): DecoderCtor | null {
  const ctor = (globalThis as Record<string, unknown>).VideoDecoder;
  return typeof ctor === "function" ? (ctor as DecoderCtor) : null;
}

export function videoDecodable(): boolean {
  return decoderCtor() !== null;
}

/** DOMException 的 `String()` 只有 `[object DOMException]`，这里取 name + message。 */
export function describeError(error: unknown): string {
  if (error && typeof error === "object") {
    const { name, message } = error as { name?: string; message?: string };
    if (message) return name ? `${name}: ${message}` : message;
  }
  return String(error);
}

/** 等一个 promise，但最多等 `ms` 毫秒；超时不算异常，交给调用方决定怎么办。 */
function waitOrTimeout<T>(
  promise: Promise<T>,
  ms: number,
): Promise<{ timedOut: boolean; value?: T; error?: unknown }> {
  return new Promise((resolve) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      resolve({ timedOut: true });
    }, ms);
    promise.then(
      (value) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        resolve({ timedOut: false, value });
      },
      (error) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        resolve({ timedOut: false, error });
      },
    );
  });
}

function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(Math.floor((hex?.length ?? 0) / 2));
  for (let index = 0; index < out.length; index += 1) {
    out[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
  }
  return out;
}

/** AVCDecoderConfigurationRecord（ISO/IEC 14496-15 §5.2.4.1）。 */
export function buildAvcC(sps: Uint8Array, pps: Uint8Array): Uint8Array {
  const out = new Uint8Array(11 + sps.length + pps.length);
  out[0] = 0x01;
  out[1] = sps[1];
  out[2] = sps[2];
  out[3] = sps[3];
  out[4] = 0xff; // lengthSizeMinusOne = 3
  out[5] = 0xe0 | 1; // numOfSequenceParameterSets = 1
  out[6] = (sps.length >> 8) & 0xff;
  out[7] = sps.length & 0xff;
  out.set(sps, 8);
  const at = 8 + sps.length;
  out[at] = 1;
  out[at + 1] = (pps.length >> 8) & 0xff;
  out[at + 2] = pps.length & 0xff;
  out.set(pps, at + 3);
  return out;
}

export interface FramePacket {
  key: boolean;
  data: Uint8Array;
}

/** 按 AUD 拆帧；没有 AUD 时退化为整段一帧。3 / 4 字节起始码都认。 */
export function splitFrames(stream: Uint8Array): FramePacket[] {
  const marks: number[] = [];
  for (let index = 0; index + 4 <= stream.length; index += 1) {
    if (stream[index] !== 0 || stream[index + 1] !== 0) continue;
    if (stream[index + 2] === 1) {
      // `00 00 01`：若前一字节也是 0，那是 `00 00 00 01` 的尾巴，已经计过。
      if (index > 0 && stream[index - 1] === 0) continue;
      if ((stream[index + 3] & 0x1f) === 9) marks.push(index);
      continue;
    }
    if (index + 5 <= stream.length && stream[index + 2] === 0 && stream[index + 3] === 1) {
      if ((stream[index + 4] & 0x1f) === 9) marks.push(index);
    }
  }
  if (!marks.length) {
    return [{ key: true, data: stream }];
  }
  const frames: FramePacket[] = [];
  for (let index = 0; index < marks.length; index += 1) {
    const from = marks[index];
    const to = index + 1 < marks.length ? marks[index + 1] : stream.length;
    const data = stream.subarray(from, to);
    frames.push({ key: hasNal(data, 5) || hasNal(data, 7), data });
  }
  return frames;
}

function hasNal(frame: Uint8Array, type: number): boolean {
  for (let index = 0; index + 3 < frame.length; index += 1) {
    const start =
      frame[index] === 0 && frame[index + 1] === 0 && frame[index + 2] === 1
        ? index + 3
        : frame[index] === 0 &&
            frame[index + 1] === 0 &&
            frame[index + 2] === 0 &&
            frame[index + 3] === 1
          ? index + 4
          : -1;
    if (start > 0 && start < frame.length && (frame[start] & 0x1f) === type) {
      return true;
    }
  }
  return false;
}

/** Annex-B → AVCC（4 字节大端长度前缀），顺带丢掉 AUD。 */
export function annexBToAvcc(frame: Uint8Array): Uint8Array {
  const parts: Uint8Array[] = [];
  let index = 0;
  let headerAt = -1;
  const flush = (end: number) => {
    if (headerAt < 0) return;
    const type = frame[headerAt] & 0x1f;
    if (type !== 9 && type !== 0) {
      parts.push(frame.subarray(headerAt, end));
    }
  };
  while (index + 3 <= frame.length) {
    if (frame[index] === 0 && frame[index + 1] === 0 && frame[index + 2] === 1) {
      flush(index);
      headerAt = index + 3;
      index = headerAt + 1;
      continue;
    }
    if (
      frame[index] === 0 &&
      frame[index + 1] === 0 &&
      frame[index + 2] === 0 &&
      index + 3 < frame.length &&
      frame[index + 3] === 1
    ) {
      flush(index);
      headerAt = index + 4;
      index = headerAt + 1;
      continue;
    }
    index += 1;
  }
  flush(frame.length);

  let total = 0;
  for (const part of parts) total += 4 + part.length;
  const out = new Uint8Array(total);
  let offset = 0;
  for (const part of parts) {
    out[offset] = (part.length >>> 24) & 0xff;
    out[offset + 1] = (part.length >>> 16) & 0xff;
    out[offset + 2] = (part.length >>> 8) & 0xff;
    out[offset + 3] = part.length & 0xff;
    out.set(part, offset + 4);
    offset += 4 + part.length;
  }
  return out;
}

export interface GopResult {
  /** 送进解码器的 chunk 数。 */
  chunks: number;
  /** 解出并交给 `onFrame` 的帧数。 */
  decoded: number;
  /** 非致命问题（例如 flush 超时，只拿到部分帧）。 */
  warning: string | null;
  /** 实际走通的路径。 */
  strategy: string;
}

/** 配置探测的等待上限：平台解码器不可用时这个 promise 可能不 settle。 */
const CONFIG_CHECK_MS = 3000;
/** 单条路径 flush 的等待上限：硬解卡住时它会一直等。 */
const FLUSH_MS = 8000;

type Strategy = "avcC" | "annexB";
const STRATEGIES: Strategy[] = ["avcC", "annexB"];

interface Attempt {
  ok: boolean;
  chunks: number;
  decoded: number;
  reason: string;
  warning: string | null;
  retryable: boolean;
}

async function runAttempt(
  strategy: Strategy,
  ctor: DecoderCtor,
  ChunkCtor: new (init: Record<string, unknown>) => unknown,
  packets: FramePacket[],
  info: UsmVideo,
  startFrame: number,
  onFrame: (frame: VideoFrame) => void,
  stage: (text: string) => void,
): Promise<Attempt> {
  let decoded = 0;
  let failure: unknown = null;
  let conversion: unknown = null;

  const decoder = new ctor({
    // 关键：帧在这里当场消费、当场关闭。攒帧会把解码器的输出缓冲耗光。
    output: (frame) => {
      const videoFrame = frame as VideoFrame;
      try {
        if (failure || conversion) return;
        onFrame(videoFrame);
        decoded += 1;
        if (decoded % 5 === 0) stage(`出帧 ${decoded}/${packets.length}（${strategy}）…`);
      } catch (error) {
        conversion = conversion ?? error;
      } finally {
        videoFrame.close();
      }
    },
    error: (error) => {
      failure = failure ?? error;
    },
  });

  const fail = (reason: string, retryable: boolean): Attempt => ({
    ok: false,
    chunks,
    decoded,
    reason,
    warning: null,
    retryable,
  });

  let chunks = 0;
  try {
    const config: Record<string, unknown> = { codec: info.codec };
    if (strategy === "avcC") {
      config.description = buildAvcC(hexToBytes(info.sps), hexToBytes(info.pps));
    }
    stage(`配置 ${info.codec}（${strategy}）…`);
    try {
      decoder.configure(config);
    } catch (error) {
      return fail(`配置失败：${describeError(error)}`, false);
    }

    for (const [index, packet] of packets.entries()) {
      const chunk = new ChunkCtor({
        type: packet.key ? "key" : "delta",
        timestamp: Math.round(((startFrame + index) * 1_000_000) / Math.max(1, info.fps)),
        data: strategy === "avcC" ? annexBToAvcc(packet.data) : packet.data,
      });
      try {
        decoder.decode(chunk);
        chunks += 1;
      } catch (error) {
        return fail(`第 ${index} 帧送包失败：${describeError(error)}`, false);
      }
      if (index === 0 || (index + 1) % 10 === 0) {
        stage(`送包 ${index + 1}/${packets.length}（${strategy}）…`);
      }
      // 每 16 包让出一次事件循环：`output` 回调是宏任务，全同步送完它一个都跑不到，
      // 帧就只能憋到最后一起吐，队列和显存都会堆着。
      if ((index + 1) % 16 === 0) {
        await new Promise<void>((resolve) => setTimeout(resolve, 0));
      }
      if (conversion) break;
    }

    stage(`等待解码器出帧（${strategy}）…`);
    const flushed = await waitOrTimeout(decoder.flush(), FLUSH_MS);
    // 停在哪一刻要看**那一刻**的队列，不是送包时的。
    const stateText = `state=${decoder.state}，队列=${decoder.decodeQueueSize ?? "?"}，已出帧=${decoded}`;
    if (flushed.error) return fail(`flush 失败：${describeError(flushed.error)}`, true);
    if (conversion) return fail(`转画面失败：${describeError(conversion)}`, false);
    if (failure) return fail(`解码报错：${describeError(failure)}`, true);
    if (flushed.timedOut) {
      if (!decoded) {
        return {
          ok: false,
          chunks,
          decoded: 0,
          reason: `${FLUSH_MS} ms 内没有出任何画面（${stateText}）`,
          warning: null,
          retryable: true,
        };
      }
      return {
        ok: true,
        chunks,
        decoded,
        reason: "",
        warning: `flush 超时，只拿到 ${decoded}/${chunks} 帧（${stateText}）`,
        retryable: false,
      };
    }
    if (!decoded) {
      return {
        ok: false,
        chunks,
        decoded: 0,
        reason: `零输出（${chunks} 个 chunk）`,
        warning: null,
        retryable: true,
      };
    }
    return { ok: true, chunks, decoded, reason: "", warning: null, retryable: false };
  } finally {
    if (decoder.state !== "closed") {
      try {
        decoder.close();
      } catch {
        // 关不掉不影响结论。
      }
    }
  }
}

/**
 * 解一个 GOP。
 *
 * `onFrame` 在解码器的 `output` 回调里**同步**调用，必须当场把帧用掉
 * （转成位图或画到画布），返回后本函数立刻 `close()` 这一帧。
 * 不要在里面 `await` —— 攒帧会让解码器等不到缓冲，画面会卡在半截。
 */
export async function decodeGop(
  stream: Uint8Array,
  info: UsmVideo,
  startFrame: number,
  onFrame: (frame: VideoFrame) => void,
  onStage?: (stage: string) => void,
): Promise<GopResult> {
  const stage = (text: string) => onStage?.(text);

  const ctor = decoderCtor();
  if (!ctor) {
    throw new UnsupportedVideoError("当前 WebView 不支持 WebCodecs 视频解码");
  }
  const sps = hexToBytes(info.sps);
  const pps = hexToBytes(info.pps);
  if (sps.length < 4 || pps.length < 1) {
    throw new UnsupportedVideoError(
      `索引里没有可用的 SPS/PPS（sps=${sps.length} B / pps=${pps.length} B）`,
    );
  }
  const packets = splitFrames(stream);
  if (!packets.length) {
    throw new UnsupportedVideoError("GOP 里没有切出任何帧");
  }
  const ChunkCtor = (globalThis as Record<string, unknown>).EncodedVideoChunk as
    | (new (init: Record<string, unknown>) => unknown)
    | undefined;
  if (!ChunkCtor) {
    throw new UnsupportedVideoError("当前 WebView 缺少 EncodedVideoChunk");
  }

  // 预检只是提前给一句人话；它本身也可能不 settle，所以只等 3 秒。
  if (typeof ctor.isConfigSupported === "function") {
    stage("探测解码能力…");
    const probe = await waitOrTimeout(
      ctor.isConfigSupported({ codec: info.codec, description: buildAvcC(sps, pps) }),
      CONFIG_CHECK_MS,
    );
    if (!probe.timedOut && probe.value && probe.value.supported === false) {
      throw new UnsupportedVideoError(`当前 WebView 不支持 ${info.codec} 解码`);
    }
  }

  const failures: string[] = [];
  for (const strategy of STRATEGIES) {
    const attempt = await runAttempt(
      strategy,
      ctor,
      ChunkCtor,
      packets,
      info,
      startFrame,
      onFrame,
      stage,
    );
    if (attempt.ok) {
      return {
        chunks: attempt.chunks,
        decoded: attempt.decoded,
        warning: attempt.warning,
        strategy,
      };
    }
    failures.push(`${strategy}：${attempt.reason}`);
    if (!attempt.retryable) break;
  }

  throw new UnsupportedVideoError(
    `视频解码失败（${info.codec} / ${info.width}×${info.height}）：${failures.join("；")}` +
      "。两条路径都没出画面，多半是这台机器的 WebView 没有可用的 H.264 解码器",
  );
}
