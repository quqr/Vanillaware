/**
 * USM 影片解码：Annex-B 分段 → AVCC → WebCodecs `VideoDecoder` → 帧序列。
 *
 * 后端按 GOP 给字节，这里拆帧、去掉 AUD、把起始码换成 4 字节长度，
 * 再用 avcC（由 SPS/PPS 拼）配置解码器。
 */

import type { UsmVideo } from "./ipc";

export class UnsupportedVideoError extends Error {}

export interface FramePacket {
  key: boolean;
  data: Uint8Array;
}

type AnyVideoDecoder = new (init: {
  output: (frame: unknown) => void;
  error: (error: unknown) => void;
}) => {
  configure: (config: Record<string, unknown>) => void;
  decode: (chunk: unknown) => void;
  flush: () => Promise<void>;
  close: () => void;
  state: string;
};

function videoDecoderCtor(): AnyVideoDecoder | null {
  const ctor = (globalThis as Record<string, unknown>).VideoDecoder;
  return typeof ctor === "function" ? (ctor as AnyVideoDecoder) : null;
}

export function videoDecodable(): boolean {
  return videoDecoderCtor() !== null;
}

function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(hex.length / 2);
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

/** 按 AUD（`00 00 00 01 09`）拆帧；没有 AUD 时退化为整段一帧。 */
export function splitFrames(stream: Uint8Array): FramePacket[] {
  const marks: number[] = [];
  for (let index = 0; index + 5 <= stream.length; index += 1) {
    if (
      stream[index] === 0 &&
      stream[index + 1] === 0 &&
      stream[index + 2] === 0 &&
      stream[index + 3] === 1 &&
      (stream[index + 4] & 0x1f) === 9
    ) {
      marks.push(index);
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
  for (let index = 0; index + 4 <= frame.length; index += 1) {
    const start =
      frame[index] === 0 && frame[index + 1] === 0 && frame[index + 2] === 1
        ? index + 3
        : frame[index] === 0 &&
            frame[index + 1] === 0 &&
            frame[index + 2] === 0 &&
            index + 3 < frame.length &&
            frame[index + 3] === 1
          ? index + 4
          : -1;
    if (start > 0 && (frame[start] & 0x1f) === type) {
      return true;
    }
  }
  return false;
}

/** Annex-B → AVCC（4 字节大端长度前缀），顺带丢掉 AUD。 */
export function annexBToAvcc(frame: Uint8Array): Uint8Array {
  const parts: Uint8Array[] = [];
  let index = 0;
  let cursor = -1;
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
      cursor = headerAt;
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
      cursor = headerAt;
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
  void cursor;
  return out;
}

/** 解一个 GOP，返回按显示顺序排列的帧（调用方负责 close）。 */
export async function decodeGop(
  stream: Uint8Array,
  info: UsmVideo,
  startFrame: number,
): Promise<VideoFrame[]> {
  const ctor = videoDecoderCtor();
  if (!ctor) {
    throw new UnsupportedVideoError("当前 WebView 不支持 WebCodecs 视频解码");
  }
  const frames: VideoFrame[] = [];
  let failure: unknown = null;
  const decoder = new ctor({
    output: (frame) => {
      frames.push(frame as VideoFrame);
    },
    error: (error) => {
      failure = error;
    },
  });

  try {
    decoder.configure({
      codec: info.codec,
      description: buildAvcC(hexToBytes(info.sps), hexToBytes(info.pps)),
      optimizeForLatency: true,
    });
    const ChunkCtor = (globalThis as Record<string, unknown>).EncodedVideoChunk as
      | (new (init: Record<string, unknown>) => unknown)
      | undefined;
    if (!ChunkCtor) {
      throw new UnsupportedVideoError("当前 WebView 缺少 EncodedVideoChunk");
    }
    const packets = splitFrames(stream);
    for (let index = 0; index < packets.length; index += 1) {
      const packet = packets[index];
      const chunk = new ChunkCtor({
        type: packet.key ? "key" : "delta",
        timestamp: Math.round(((startFrame + index) / Math.max(1, info.fps)) * 1_000_000),
        data: annexBToAvcc(packet.data),
      });
      decoder.decode(chunk);
    }
    await decoder.flush();
  } finally {
    if (decoder.state !== "closed") {
      decoder.close();
    }
  }

  if (failure) {
    frames.forEach((frame) => frame.close());
    throw new UnsupportedVideoError(`视频解码失败：${String(failure)}`);
  }
  return frames;
}
