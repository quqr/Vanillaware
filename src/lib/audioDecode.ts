/**
 * 音频片段解码：把后端给的载荷变成可绘制的浮点声道。
 *
 * 两条路径：
 * - `hca`：后端已用 cridecoder 解成交错 i16 PCM，这里只做定点转浮点；
 * - `lopus`：后端只拆出裸 Opus 包，这里用 WebCodecs `AudioDecoder` 解码。
 *
 * WebCodecs 不可用时抛 `UnsupportedAudioError`，由查看器降级成「只能看结构」。
 */

import type { AudioClip } from "./ipc";

export class UnsupportedAudioError extends Error {}

export interface DecodedAudio {
  sampleRate: number;
  channels: Float32Array<ArrayBuffer>[];
  /** 每声道样本数。 */
  frames: number;
  durationSec: number;
  /** 丢掉了多少前导样本（Opus pre-skip）。 */
  skipped: number;
}

/** 每包样本数：lopus 固定 20 ms @ 48 kHz。 */
const OPUS_PACKET_SAMPLES = 960;
const OPUS_PACKET_MICROS = 20_000;

/**
 * 单次解码上限（秒）。语料里最长的是 107 秒的 BGM，
 * 留 5 分钟余量，超过就只解前 5 分钟，免得几十兆浮点把页面拖死。
 */
const MAX_CLIP_SECONDS = 300;

type AnyAudioDecoder = new (init: {
  output: (frame: unknown) => void;
  error: (error: unknown) => void;
}) => {
  configure: (config: Record<string, unknown>) => void;
  decode: (chunk: unknown) => void;
  flush: () => Promise<void>;
  close: () => void;
  state: string;
};

function audioDecoderCtor(): AnyAudioDecoder | null {
  const ctor = (globalThis as Record<string, unknown>).AudioDecoder;
  return typeof ctor === "function" ? (ctor as AnyAudioDecoder) : null;
}

/** WebCodecs `AudioData` 的字段（lib.dom 未覆盖，自己声明）。 */
interface AudioFrameLike {
  numberOfFrames: number;
  numberOfChannels: number;
  format: string;
  /** 微秒。 */
  timestamp: number;
  /**
   * WebCodecs 的 `copyTo(destination, options)` 第二个参数是**必填**的：
   * `planeIndex` 为 required member，缺了它 Chrome 直接抛
   * `TypeError: 2 arguments required, but only 1 present`。
   * 这里把 `options` 声明成必填，让「漏传」在编译期就报错。
   */
  copyTo: (
    target: Float32Array | Int16Array | Int32Array | Uint8Array,
    options: { planeIndex: number; frameOffset?: number; frameCount?: number; format?: string },
  ) => void;
  close: () => void;
}

type SampleArray = Float32Array | Int16Array | Int32Array | Uint8Array;

/** `copyTo` 要求目标数组类型与 `format` 一致，不能一律用 Float32Array。 */
function sampleArray(format: string, length: number): SampleArray {
  switch (format.replace("-planar", "")) {
    case "s16":
      return new Int16Array(length);
    case "s32":
      return new Int32Array(length);
    case "u8":
      return new Uint8Array(length);
    default:
      return new Float32Array(length);
  }
}

function sampleAt(array: SampleArray, index: number): number {
  const value = array[index];
  if (array instanceof Int16Array) return value / 32768;
  if (array instanceof Int32Array) return value / 2147483648;
  if (array instanceof Uint8Array) return (value - 128) / 128;
  return value;
}

/** 把一帧 `AudioData` 拷进按时间戳定位的浮点声道。 */
function copyPlanes(frame: AudioFrameLike, planes: Float32Array[], start: number, channels: number) {
  const format = String(frame.format);
  const sourceChannels = Math.max(1, frame.numberOfChannels);
  const frameCount = frame.numberOfFrames;
  if (format.includes("planar")) {
    for (let channel = 0; channel < channels; channel += 1) {
      const buffer = sampleArray(format, frameCount);
      frame.copyTo(buffer, { planeIndex: Math.min(channel, sourceChannels - 1) });
      const plane = planes[channel];
      for (let index = 0; index < frameCount; index += 1) {
        plane[start + index] = sampleAt(buffer, index);
      }
    }
    return;
  }
  // 交错格式只有 1 个 plane，planeIndex 必须显式给 0（不能省参数）。
  const buffer = sampleArray(format, frameCount * sourceChannels);
  frame.copyTo(buffer, { planeIndex: 0 });
  for (let index = 0; index < frameCount; index += 1) {
    for (let channel = 0; channel < channels; channel += 1) {
      planes[channel][start + index] =
        sampleAt(buffer, index * sourceChannels + Math.min(channel, sourceChannels - 1));
    }
  }
}

/** 当前 WebView 是否支持 WebCodecs 音频解码。 */
export function opusDecodable(): boolean {
  return audioDecoderCtor() !== null;
}

export async function decodeClip(clip: AudioClip, data: ArrayBuffer): Promise<DecodedAudio> {
  if (clip.codec === "hca") {
    return decodePcm(clip, data);
  }
  if (clip.codec === "lopus") {
    return decodeOpus(clip, data);
  }
  throw new UnsupportedAudioError(`未知编码 ${clip.codec}`);
}

/** 交错 i16 → 分声道 f32。 */
function decodePcm(clip: AudioClip, data: ArrayBuffer): DecodedAudio {
  const channels = Math.max(1, clip.channels);
  const interleaved = new Int16Array(data);
  const limit = clip.sampleRate * MAX_CLIP_SECONDS;
  const frames = Math.min(Math.floor(interleaved.length / channels), limit);
  const planes = Array.from({ length: channels }, () => new Float32Array(frames));
  for (let frame = 0; frame < frames; frame += 1) {
    const base = frame * channels;
    for (let channel = 0; channel < channels; channel += 1) {
      planes[channel][frame] = interleaved[base + channel] / 32768;
    }
  }
  return {
    sampleRate: clip.sampleRate,
    channels: planes,
    frames,
    durationSec: frames / Math.max(1, clip.sampleRate),
    skipped: 0,
  };
}

async function decodeOpus(clip: AudioClip, data: ArrayBuffer): Promise<DecodedAudio> {
  const ctor = audioDecoderCtor();
  if (!ctor) {
    throw new UnsupportedAudioError("当前 WebView 不支持 WebCodecs 音频解码");
  }
  const channels = Math.max(1, clip.channels);
  const stride = clip.stride;
  if (!stride) {
    throw new UnsupportedAudioError("lopus 缺少帧步长");
  }
  const packets = new Uint8Array(data);
  // 包上限按**微秒**算：包长是 20 000 µs，不是 20 000 ms。
  const capped = Math.floor((MAX_CLIP_SECONDS * 1_000_000) / OPUS_PACKET_MICROS);
  const count = Math.min(Math.floor(packets.length / stride), capped);
  if (!count) {
    throw new UnsupportedAudioError("lopus 没有可解码的包");
  }

  const total = count * OPUS_PACKET_SAMPLES;
  const sampleRate = Math.max(1, clip.sampleRate);
  const planes = Array.from({ length: channels }, () => new Float32Array(total));
  let written = 0;
  let outputs = 0;
  let failure: unknown = null;

  const decoder = new ctor({
    output: (frame) => {
      const typed = frame as AudioFrameLike;
      try {
        const frameCount = typed.numberOfFrames;
        if (frameCount <= 0) return;
        outputs += 1;
        // 用输出时间戳定位，不假设解码器按顺序、无丢帧地吐帧。
        const start = Number.isFinite(typed.timestamp)
          ? Math.round((typed.timestamp / 1e6) * sampleRate)
          : written;
        if (start < 0 || start + frameCount > total) return;
        copyPlanes(typed, planes, start, channels);
        written = Math.max(written, start + frameCount);
      } catch (error) {
        failure = failure ?? error;
      } finally {
        typed.close();
      }
    },
    error: (error) => {
      failure = error;
    },
  });

  try {
    decoder.configure({
      codec: "opus",
      sampleRate: clip.sampleRate,
      numberOfChannels: channels,
    });
    const ChunkCtor = (globalThis as Record<string, unknown>).EncodedAudioChunk as
      | (new (init: Record<string, unknown>) => unknown)
      | undefined;
    if (!ChunkCtor) {
      throw new UnsupportedAudioError("当前 WebView 缺少 EncodedAudioChunk");
    }
    for (let index = 0; index < count; index += 1) {
      const chunk = new ChunkCtor({
        type: "key",
        timestamp: index * OPUS_PACKET_MICROS,
        data: packets.subarray(index * stride, (index + 1) * stride),
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
    throw new UnsupportedAudioError(`Opus 解码失败：${String(failure)}`);
  }
  if (written === 0) {
    throw new UnsupportedAudioError(
      `Opus 解码没有产出样本（${count} 包 / 解码器输出 ${outputs} 帧）`,
    );
  }

  // 裁掉 pre-skip 与尾部未写满的区域。
  const skip = Math.min(clip.skip, written);
  const frames = written - skip;
  const trimmed = planes.map((plane) => plane.subarray(skip, skip + frames));
  return {
    sampleRate: clip.sampleRate,
    channels: trimmed.map((plane) => plane.slice()),
    frames,
    durationSec: frames / Math.max(1, clip.sampleRate),
    skipped: skip,
  };
}

/** 波形包络：每列一对 (min, max)。 */
export function buildPeaks(channels: Float32Array[], columns: number): Float32Array {
  const data = channels[0] ?? new Float32Array(0);
  const peaks = new Float32Array(columns * 2);
  if (!data.length || columns <= 0) {
    return peaks;
  }
  const perColumn = data.length / columns;
  for (let column = 0; column < columns; column += 1) {
    const from = Math.floor(column * perColumn);
    const to = Math.min(data.length, Math.max(from + 1, Math.floor((column + 1) * perColumn)));
    let min = 0;
    let max = 0;
    for (let index = from; index < to; index += 1) {
      const value = data[index];
      if (value < min) min = value;
      if (value > max) max = value;
    }
    peaks[column * 2] = min;
    peaks[column * 2 + 1] = max;
  }
  return peaks;
}
