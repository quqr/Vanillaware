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
  const capped = Math.ceil((MAX_CLIP_SECONDS * 1000) / OPUS_PACKET_MICROS);
  const count = Math.min(Math.floor(packets.length / stride), capped);
  if (!count) {
    throw new UnsupportedAudioError("lopus 没有可解码的包");
  }

  const total = count * OPUS_PACKET_SAMPLES;
  const planes = Array.from({ length: channels }, () => new Float32Array(total));
  let written = 0;
  let failure: unknown = null;

  const decoder = new ctor({
    output: (frame) => {
      const typed = frame as {
        numberOfFrames: number;
        numberOfChannels: number;
        format: string;
        copyTo: (target: Float32Array, options?: Record<string, unknown>) => void;
        close: () => void;
      };
      const frameCount = typed.numberOfFrames;
      if (frameCount > 0 && written + frameCount <= total) {
        const planar = String(typed.format).includes("planar");
        if (planar) {
          for (let channel = 0; channel < channels; channel += 1) {
            const tmp = new Float32Array(frameCount);
            typed.copyTo(tmp, { planeIndex: Math.min(channel, typed.numberOfChannels - 1) });
            planes[channel].set(tmp, written);
          }
        } else {
          const tmp = new Float32Array(frameCount * typed.numberOfChannels);
          typed.copyTo(tmp);
          for (let frame = 0; frame < frameCount; frame += 1) {
            for (let channel = 0; channel < channels; channel += 1) {
              planes[channel][written + frame] =
                tmp[frame * typed.numberOfChannels + Math.min(channel, typed.numberOfChannels - 1)];
            }
          }
        }
        written += frameCount;
      }
      typed.close();
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
    throw new UnsupportedAudioError("Opus 解码没有产出样本");
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
