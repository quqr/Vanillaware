export const AUDIO_FORMATS = ["f32-planar", "f32", "s16"];

const AUDIO_ARITY =
  "Failed to execute 'copyTo' on 'AudioData': 2 arguments required, but only 1 present.";
const AUDIO_NO_PLANE =
  "Failed to execute 'copyTo' on 'AudioData': Failed to read the 'planeIndex' property from 'AudioDataCopyToOptions': Required member is undefined.";

const audioUnit = (globalIndex, channel) =>
  Math.fround(((globalIndex * 3 + channel * 7) % 1024) / 1024 - 0.5);

const AUDIO_CODECS = {
  "f32-planar": { kind: "f32", store: audioUnit, restore: (v) => v },
  f32: { kind: "f32", store: audioUnit, restore: (v) => v },
  s16: {
    kind: "s16",
    store: (g, ch) => Math.round(audioUnit(g, ch) * 32768),
    restore: (v) => v / 32768,
  },
};

export function audioLayout(format) {
  const spec = AUDIO_CODECS[format];
  if (!spec) throw new Error(`unknown audio format ${format}`);
  return spec;
}

export function installAudioDecoder({
  format,
  framesPerPacket,
  packetMicros,
  stride,
  stats,
}) {
  const spec = audioLayout(format);
  const SampleCtor = {
    u8: Uint8Array,
    s16: Int16Array,
    s32: Int32Array,
    f32: Float32Array,
  }[spec.kind];

  class MockAudioData {
    constructor({ numberOfFrames, numberOfChannels, timestamp, base }) {
      this.format = format;
      this.numberOfFrames = numberOfFrames;
      this.numberOfChannels = numberOfChannels;
      this.timestamp = timestamp;
      this.sampleRate = 48000;
      this._base = base;
    }

    copyTo(destination, options) {
      if (arguments.length < 2) throw new TypeError(AUDIO_ARITY);
      if (options === null || typeof options !== "object") {
        throw new TypeError("AudioDataCopyToOptions must be an object");
      }
      const planeIndex = options.planeIndex;
      if (planeIndex === undefined) throw new TypeError(AUDIO_NO_PLANE);
      const planar = this.format.endsWith("-planar");
      const planes = planar ? this.numberOfChannels : 1;
      if (planeIndex < 0 || planeIndex >= planes) {
        throw new RangeError(`planeIndex ${planeIndex} out of ${planes} planes`);
      }
      const offset = options.frameOffset ?? 0;
      const count = options.frameCount ?? this.numberOfFrames - offset;
      const need = planar ? count : count * this.numberOfChannels;
      if (!(destination instanceof SampleCtor)) {
        throw new TypeError(`destination must be ${SampleCtor.name}`);
      }
      if (destination.length < need) {
        throw new TypeError(`destination too short: need ${need}, got ${destination.length}`);
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

  stats.decoded = 0;
  globalThis.AudioDecoder = class {
    constructor(init) {
      this._output = init.output;
      this.state = "unconfigured";
    }

    configure(config) {
      if (config.codec !== "opus") throw new TypeError(`unsupported codec ${config.codec}`);
      if (!(config.sampleRate > 0) || !(config.numberOfChannels > 0)) {
        throw new TypeError("configure needs sampleRate and numberOfChannels");
      }
      this.config = config;
      this.state = "configured";
    }

    decode(chunk) {
      if (this.state !== "configured") throw new Error("decoder not configured");
      if (chunk.byteLength !== stride) {
        throw new Error(`packet length ${chunk.byteLength} != stride ${stride}`);
      }
      stats.decoded += 1;
      this._output(
        new MockAudioData({
          numberOfFrames: framesPerPacket,
          numberOfChannels: this.config.numberOfChannels,
          timestamp: chunk.timestamp,
          base: Math.round(chunk.timestamp / packetMicros) * framesPerPacket,
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

export function parseAvcC(bytes) {
  if (bytes[0] !== 1) throw new TypeError(`avcC configurationVersion=${bytes[0]} (expected 1)`);
  const lengthSize = (bytes[4] & 0x03) + 1;
  const spsCount = bytes[5] & 0x1f;
  if (spsCount < 1) throw new TypeError("avcC has no SPS");
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
  if (at !== bytes.length) throw new TypeError(`avcC has ${bytes.length - at} trailing bytes`);
  if (!ppsList.length) throw new TypeError("avcC has no PPS");
  return { lengthSize, sps: spsList[0], pps: ppsList[0] };
}

export function parseAvcc(data, lengthSize) {
  const nals = [];
  let at = 0;
  while (at < data.length) {
    if (at + lengthSize > data.length) throw new TypeError(`AVCC length field out of range @${at}`);
    let len = 0;
    for (let i = 0; i < lengthSize; i += 1) len = (len << 8) | data[at + i];
    at += lengthSize;
    if (len <= 0 || at + len > data.length) {
      throw new TypeError(`AVCC length ${len} out of range (${data.length - at} left)`);
    }
    nals.push(data.subarray(at, at + len));
    at += len;
  }
  return nals;
}

export function splitAnnexB(data) {
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
  if (at < 0) throw new TypeError("Annex-B chunk has no start code");
  nals.push(data.subarray(at));
  return nals.filter((nal) => nal.length);
}

export function installVideoDecoder({ width, height, stats }) {
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
      if (this.state === "closed") throw new TypeError("decoder already closed");
      if (!/^avc1\.[0-9a-f]{6}$/i.test(config.codec ?? "")) {
        throw new TypeError(`invalid codec string ${config.codec}`);
      }
      if (!config.description) {
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
      if (avcC.sps[3] !== level) {
        throw new TypeError(`codec level ${level} != SPS ${avcC.sps[3]}`);
      }
      stats.configured = { codec: config.codec, lengthSize: avcC.lengthSize };
      this._avcC = avcC;
      this.state = "configured";
    }

    decode(chunk) {
      if (this.state !== "configured") {
        this._error(new Error("decoder not configured"));
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
        this._error(new Error("chunk still carries an AUD (type 9)"));
        return;
      }
      if (chunk.type === "key") {
        stats.keyChunks += 1;
        if (!types.includes(5)) {
          this._error(new Error(`key chunk has no IDR: ${types.join(",")}`));
          return;
        }
      }
      stats.decoded += 1;
      this._output({
        width,
        height,
        timestamp: chunk.timestamp,
        closed: false,
        close() {
          this.closed = true;
        },
      });
    }

    async flush() {
      if (this.state === "closed") throw new Error("flush after close");
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
}
