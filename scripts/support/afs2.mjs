export const CHUNK_BASIC = 0x80000001;
export const CHUNK_DATA = 0x80000004;
export const FRAMES_PER_PACKET = 960;
export const PACKET_MICROS = 20_000;

export function lopusTracks(buf) {
  if (buf.toString("latin1", 0, 4) !== "AFS2") throw new Error("not an AFS2 container");
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

export function materializeLopus(buf, track) {
  const packets = Buffer.allocUnsafe(track.total * track.stride);
  for (let k = 0; k < track.total; k += 1) {
    const at = track.base + k * track.frameSize;
    buf.copy(packets, k * track.stride, at, at + track.stride);
  }
  return packets;
}
