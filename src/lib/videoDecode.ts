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

export function describeError(error: unknown): string {
  if (error && typeof error === "object") {
    const { name, message } = error as { name?: string; message?: string };
    if (message) return name ? `${name}: ${message}` : message;
  }
  return String(error);
}

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

export function splitFrames(stream: Uint8Array): FramePacket[] {
  const marks: number[] = [];
  for (let index = 0; index + 4 <= stream.length; index += 1) {
    if (stream[index] !== 0 || stream[index + 1] !== 0) continue;
    if (stream[index + 2] === 1) {
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

  chunks: number;

  decoded: number;

  warning: string | null;

  strategy: string;
}

const CONFIG_CHECK_MS = 3000;

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
    output: (frame) => {
      const videoFrame = frame as VideoFrame;
      try {
        if (failure || conversion) return;
        onFrame(videoFrame);
        decoded += 1;
        if (decoded % 5 === 0) stage(`decoded ${decoded}/${packets.length} (${strategy})…`);
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
    stage(`configuring ${info.codec} (${strategy})…`);
    try {
      decoder.configure(config);
    } catch (error) {
      return fail(`configure failed: ${describeError(error)}`, false);
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
        return fail(`feeding frame ${index} failed: ${describeError(error)}`, false);
      }
      if (index === 0 || (index + 1) % 10 === 0) {
        stage(`fed ${index + 1}/${packets.length} (${strategy})…`);
      }
      if ((index + 1) % 16 === 0) {
        await new Promise<void>((resolve) => setTimeout(resolve, 0));
      }
      if (conversion) break;
    }

    stage(`waiting for output (${strategy})…`);
    const flushed = await waitOrTimeout(decoder.flush(), FLUSH_MS);
    const stateText = `state=${decoder.state}, queued=${decoder.decodeQueueSize ?? "?"}, emitted=${decoded}`;
    if (flushed.error) return fail(`flush failed: ${describeError(flushed.error)}`, true);
    if (conversion) return fail(`converting a frame failed: ${describeError(conversion)}`, false);
    if (failure) return fail(`decoder error: ${describeError(failure)}`, true);
    if (flushed.timedOut) {
      if (!decoded) {
        return {
          ok: false,
          chunks,
          decoded: 0,
          reason: `no picture within ${FLUSH_MS} ms (${stateText})`,
          warning: null,
          retryable: true,
        };
      }
      return {
        ok: true,
        chunks,
        decoded,
        reason: "",
        warning: `flush timed out, only ${decoded}/${chunks} frames (${stateText})`,
        retryable: false,
      };
    }
    if (!decoded) {
      return {
        ok: false,
        chunks,
        decoded: 0,
        reason: `no output (${chunks} chunks)`,
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
      }
    }
  }
}

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
    throw new UnsupportedVideoError("this WebView has no WebCodecs video decoding");
  }
  const sps = hexToBytes(info.sps);
  const pps = hexToBytes(info.pps);
  if (sps.length < 4 || pps.length < 1) {
    throw new UnsupportedVideoError(
      `the index carries no usable SPS/PPS (sps=${sps.length} B / pps=${pps.length} B)`,
    );
  }
  const packets = splitFrames(stream);
  if (!packets.length) {
    throw new UnsupportedVideoError("no frame could be split out of this GOP");
  }
  const ChunkCtor = (globalThis as Record<string, unknown>).EncodedVideoChunk as
    | (new (init: Record<string, unknown>) => unknown)
    | undefined;
  if (!ChunkCtor) {
    throw new UnsupportedVideoError("this WebView has no EncodedVideoChunk");
  }

  if (typeof ctor.isConfigSupported === "function") {
    stage("probing decode support…");
    const probe = await waitOrTimeout(
      ctor.isConfigSupported({ codec: info.codec, description: buildAvcC(sps, pps) }),
      CONFIG_CHECK_MS,
    );
    if (!probe.timedOut && probe.value && probe.value.supported === false) {
      throw new UnsupportedVideoError(`this WebView cannot decode ${info.codec}`);
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
    failures.push(`${strategy}: ${attempt.reason}`);
    if (!attempt.retryable) break;
  }

  throw new UnsupportedVideoError(
    `video decoding failed (${info.codec} / ${info.width}×${info.height}): ${failures.join("; ")}` +
      ". neither path produced a picture, so this machine likely has no H.264 decoder available",
  );
}
