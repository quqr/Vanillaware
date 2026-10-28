//! 音频容器解析与解码（ACB / AWB）。
//!
//! 两种载荷：`.hca`（CRI HCA，走 `cridecoder` 解成 PCM）与 `.lopus`（CRI Opus，
//! 每帧固定 `frame_size` 字节，前 8 字节是帧头，其后是裸 Opus 包，交给前端 WebCodecs 解码）。
//!
//! 列表阶段只解析表，不读音频字节：`voice_*.acb` 有一万三千条 cue，
//! 逐条探测头会把首次打开拖到秒级。字节按需 `read_track` 从 AWB 里 seek 出来。

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use cridecoder::acb::{AfsArchive, TrackList, UtfTable, Value};
use cridecoder::HcaDecoder;

/// lopus 每帧前 8 字节是帧头（`00 00 00 f0` + 4 字节载荷标记），其后才是 Opus 包。
pub const LOPUS_FRAME_HEADER: usize = 8;

/// basic info chunk 的标签。
const CHUNK_BASIC: u32 = 0x8000_0001;
/// data chunk 的标签。
const CHUNK_DATA: u32 = 0x8000_0004;

#[derive(Debug)]
pub enum AudioError {
    Io(std::io::Error),
    Acb(String),
    Malformed(String),
    NoTracks,
    OutOfRange(usize),
    Decode(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::Io(error) => write!(f, "IO: {error}"),
            AudioError::Acb(message) => write!(f, "ACB: {message}"),
            AudioError::Malformed(message) => write!(f, "容器损坏：{message}"),
            AudioError::NoTracks => write!(f, "该容器没有可播放的轨道"),
            AudioError::OutOfRange(index) => write!(f, "轨道 {index} 越界"),
            AudioError::Decode(message) => write!(f, "解码失败：{message}"),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<std::io::Error> for AudioError {
    fn from(error: std::io::Error) -> Self {
        AudioError::Io(error)
    }
}

/// 载荷编码。`Other` 保留原始扩展名（CRI 的 `enc_type` 未知时）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Codec {
    Hca,
    Lopus,
    Adx,
    Other(String),
}

impl Codec {
    /// 列表里的短标签。
    pub fn tag(&self) -> String {
        match self {
            Codec::Hca => "HCA".to_string(),
            Codec::Lopus => "Opus".to_string(),
            Codec::Adx => "ADX".to_string(),
            Codec::Other(ext) => ext.to_uppercase(),
        }
    }

    /// 是否能在应用内解码播放。
    pub fn playable(&self) -> bool {
        matches!(self, Codec::Hca | Codec::Lopus)
    }

    fn from_wave_ext(ext: &str) -> Self {
        match ext.to_ascii_lowercase().trim_start_matches('.') {
            "hca" => Codec::Hca,
            "lopus" | "opus" => Codec::Lopus,
            "adx" => Codec::Adx,
            other => Codec::Other(other.to_string()),
        }
    }

    /// 用数据头兜底判定（AWB 没有 `enc_type`）。
    fn sniff(bytes: &[u8]) -> Self {
        if bytes.starts_with(b"HCA\0") {
            return Codec::Hca;
        }
        if bytes.len() >= 4
            && bytes[0] == 0x01
            && bytes[1] == 0x00
            && bytes[2] == 0x00
            && bytes[3] == 0x80
        {
            return Codec::Lopus;
        }
        if bytes.len() >= 2 && bytes[0] == 0x80 && bytes[1] == 0x00 {
            return Codec::Adx;
        }
        Codec::Other("bin".to_string())
    }
}

/// 一条轨道（列表用，不含音频字节）。
#[derive(Clone, Debug)]
pub struct TrackInfo {
    /// 容器内的顺序号，取字节时按它定位。
    pub index: usize,
    pub name: String,
    pub cue_id: i32,
    pub codec: Codec,
    /// 在外部 AWB 里（`stream_awb_id` 指向 `external`）。
    pub stream: bool,
    pub stream_awb_id: i32,
    pub wav_id: i32,
    pub bytes: u64,
}

/// 解析后的容器索引。
pub struct Bundle {
    pub source: PathBuf,
    pub container: &'static str,
    pub tracks: Vec<TrackInfo>,
    pub external: Vec<PathBuf>,
    pub warnings: Vec<String>,
    embedded_awb: Option<Vec<u8>>,
}

impl Bundle {
    /// 取某条轨道的原始载荷字节。
    pub fn read_track(&self, index: usize) -> Result<Vec<u8>, AudioError> {
        let track = self
            .tracks
            .get(index)
            .ok_or(AudioError::OutOfRange(index))?;
        if self.container == "awb" {
            let mut archive = open_afs(&self.source)?;
            return archive
                .file_data_for_cue_id(track.cue_id)
                .map_err(|error| AudioError::Malformed(error.to_string()));
        }
        if track.stream {
            let path = self
                .external
                .get(track.stream_awb_id.max(0) as usize)
                .ok_or_else(|| {
                    AudioError::Malformed(format!(
                        "轨道 {} 指向外部 AWB #{}，但只找到 {} 个",
                        track.name,
                        track.stream_awb_id,
                        self.external.len()
                    ))
                })?;
            let mut archive = open_afs(path)?;
            return archive
                .file_data_for_cue_id(track.wav_id)
                .map_err(|error| AudioError::Malformed(error.to_string()));
        }
        let embedded = self
            .embedded_awb
            .as_ref()
            .ok_or_else(|| AudioError::Malformed(format!("轨道 {} 没有内嵌 AWB", track.name)))?;
        let mut archive = AfsArchive::new(Cursor::new(embedded.as_slice()))
            .map_err(|error| AudioError::Malformed(error.to_string()))?;
        archive
            .file_data_for_cue_id(track.wav_id)
            .map_err(|error| AudioError::Malformed(error.to_string()))
    }
}

fn open_afs(path: &Path) -> Result<AfsArchive<BufReader<File>>, AudioError> {
    let file = BufReader::new(File::open(path)?);
    AfsArchive::new(file).map_err(|error| AudioError::Malformed(error.to_string()))
}

/// 解析 `.acb`：cue 表 → 轨道列表；数据在内嵌 AWB 或同目录 `{Name}.awb`。
pub fn open_acb(path: &Path) -> Result<Bundle, AudioError> {
    let file = BufReader::new(File::open(path)?);
    let utf = UtfTable::new(file).map_err(|error| AudioError::Acb(error.to_string()))?;
    let tracks = TrackList::new(&utf).map_err(|error| AudioError::Acb(error.to_string()))?;

    let mut warnings = Vec::new();
    let row = utf.rows.first().ok_or(AudioError::NoTracks)?;

    let embedded_awb = row
        .get("AwbFile")
        .and_then(Value::as_bytes)
        .filter(|bytes| !bytes.is_empty())
        .map(<[u8]>::to_vec);

    let external = {
        let mut paths = Vec::new();
        if let Some(hash) = row.get("StreamAwbHash").and_then(Value::as_bytes) {
            if !hash.is_empty() {
                if let Ok(table) = UtfTable::new(Cursor::new(hash)) {
                    if let Some(dir) = path.parent() {
                        for awb_row in &table.rows {
                            if let Some(name) = awb_row.get("Name").and_then(Value::as_string) {
                                paths.push(dir.join(format!("{name}.awb")));
                            }
                        }
                    }
                }
            }
        }
        paths
    };
    if tracks.tracks.is_empty() {
        return Err(AudioError::NoTracks);
    }
    if embedded_awb.is_none() && external.is_empty() {
        warnings.push("ACB 既没有内嵌 AWB，也没有同目录的流式 AWB".to_string());
    }

    let mut seen = std::collections::BTreeSet::new();
    for wave in &external {
        if !wave.exists() {
            warnings.push(format!("缺少外部 AWB：{}", wave.display()));
        } else if !seen.insert(wave.clone()) {
            // 同一 AWB 被两个 acb 共用时不重复提示。
        }
    }

    let list = tracks
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| TrackInfo {
            index,
            name: track.name.clone(),
            cue_id: track.cue_id,
            codec: Codec::from_wave_ext(&cridecoder::acb::wave_type_extension(track.enc_type)),
            stream: track.is_stream,
            stream_awb_id: track.stream_awb_id,
            wav_id: track.wav_id,
            bytes: 0,
        })
        .collect();

    Ok(Bundle {
        source: path.to_path_buf(),
        container: "acb",
        tracks: list,
        external,
        warnings,
        embedded_awb,
    })
}

/// 解析 `.awb`：AFS2 条目即轨道，名字用 id 表示。
pub fn open_awb(path: &Path) -> Result<Bundle, AudioError> {
    let archive = open_afs(path)?;
    if archive.files.is_empty() {
        return Err(AudioError::NoTracks);
    }
    // 同一 AWB 通常统一编码：只看第一条，避免对一万三千条逐条 seek。
    let first = archive.files[0];
    let hint = {
        let mut archive = open_afs(path)?;
        match archive.file_data(first) {
            Ok(bytes) => Codec::sniff(&bytes),
            Err(_) => Codec::Other("?".to_string()),
        }
    };
    let tracks = archive
        .files
        .iter()
        .enumerate()
        .map(|(index, entry)| TrackInfo {
            index,
            name: format!("#{}", entry.cue_id),
            cue_id: entry.cue_id,
            codec: hint.clone(),
            stream: false,
            stream_awb_id: -1,
            wav_id: entry.cue_id,
            bytes: entry.size as u64,
        })
        .collect();

    Ok(Bundle {
        source: path.to_path_buf(),
        container: "awb",
        tracks,
        external: Vec::new(),
        warnings: Vec::new(),
        embedded_awb: None,
    })
}

/// 解码结果。
pub enum Clip {
    /// HCA：交错 i16 PCM。
    Pcm {
        sample_rate: u32,
        channels: u32,
        frames: u64,
        samples: Vec<i16>,
    },
    /// lopus：裸 Opus 包按固定步长拼接，交给 WebCodecs。
    OpusPackets {
        sample_rate: u32,
        channels: u32,
        frames: u64,
        /// 每包字节数（已去掉 8 字节帧头）。
        stride: u32,
        /// 需要丢弃的前导样本（Opus pre-skip）。
        skip: u32,
        packets: Vec<u8>,
    },
}

/// 读一条轨道并解码。
pub fn decode_track(bundle: &Bundle, index: usize) -> Result<Clip, AudioError> {
    let bytes = bundle.read_track(index)?;
    let codec = bundle
        .tracks
        .get(index)
        .map(|track| track.codec.clone())
        .unwrap_or_else(|| Codec::sniff(&bytes));
    match codec {
        Codec::Hca => decode_hca(&bytes),
        Codec::Lopus => parse_lopus(&bytes),
        other => {
            let sniffed = Codec::sniff(&bytes);
            match sniffed {
                Codec::Hca => decode_hca(&bytes),
                Codec::Lopus => parse_lopus(&bytes),
                _ => Err(AudioError::Decode(format!(
                    "{} 暂不支持应用内解码",
                    other.tag()
                ))),
            }
        }
    }
}

fn decode_hca(bytes: &[u8]) -> Result<Clip, AudioError> {
    let mut decoder = HcaDecoder::from_reader(Cursor::new(bytes))
        .map_err(|error| AudioError::Decode(error.to_string()))?;
    let info = decoder.info().clone();
    let pcm = decoder
        .decode_all()
        .map_err(|error| AudioError::Decode(error.to_string()))?;
    let channels = info.channel_count.max(1);
    let frames = (pcm.len() / channels as usize) as u64;
    let samples = pcm
        .iter()
        .map(|value| (value.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();
    Ok(Clip::Pcm {
        sample_rate: info.sampling_rate,
        channels,
        frames,
        samples,
    })
}

/// lopus：`0x80000001` 基本块 + `0x80000004` 数据块；数据块里是定长帧，
/// 每帧前 8 字节为帧头，其后是裸 Opus 包。
fn parse_lopus(bytes: &[u8]) -> Result<Clip, AudioError> {
    if bytes.len() < 0x20 {
        return Err(AudioError::Malformed("lopus 头不足 0x20 字节".to_string()));
    }
    if u32_at(bytes, 0) != CHUNK_BASIC {
        return Err(AudioError::Malformed(format!(
            "lopus 缺少基本块（读到 {:#x}）",
            u32_at(bytes, 0)
        )));
    }
    let channels = bytes[0x09] as u32;
    let frame_size = u16_at(bytes, 0x0a) as usize;
    let sample_rate = u32_at(bytes, 0x0c);
    let data_offset = u32_at(bytes, 0x10) as usize;
    let skip = u16_at(bytes, 0x1c) as u32;
    if frame_size <= LOPUS_FRAME_HEADER || sample_rate == 0 {
        return Err(AudioError::Malformed(format!(
            "lopus 参数不成立：frame_size={frame_size} sample_rate={sample_rate}"
        )));
    }
    if u32_at(bytes, data_offset) != CHUNK_DATA {
        return Err(AudioError::Malformed(format!(
            "{data_offset:#x} 处不是数据块"
        )));
    }
    let data_size = u32_at(bytes, data_offset + 4) as usize;
    let start = data_offset + 8;
    let end = start.saturating_add(data_size).min(bytes.len());
    let body = &bytes[start..end];
    let stride = frame_size - LOPUS_FRAME_HEADER;
    let count = body.len() / frame_size;
    let mut packets = Vec::with_capacity(count * stride);
    for index in 0..count {
        let frame = index * frame_size;
        let from = frame + LOPUS_FRAME_HEADER;
        packets.extend_from_slice(&body[from..frame + frame_size]);
    }
    Ok(Clip::OpusPackets {
        sample_rate,
        channels: channels.max(1),
        frames: count as u64 * 960,
        stride: stride as u32,
        skip,
        packets,
    })
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    bytes
        .get(at..at + 4)
        .map(|slice| u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
        .unwrap_or(0)
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    bytes
        .get(at..at + 2)
        .map(|slice| u16::from_le_bytes([slice[0], slice[1]]))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造最小 lopus：两块 + 两帧定长包。
    fn build_lopus() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&CHUNK_BASIC.to_le_bytes());
        bytes.extend_from_slice(&0x18u32.to_le_bytes());
        bytes.extend_from_slice(&[0x00, 0x02, 0xf8, 0x00]); // version, channels, frame_size
        bytes.extend_from_slice(&48000u32.to_le_bytes());
        bytes.extend_from_slice(&0x20u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&120u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());

        let frame_size = 248usize;
        let frames = 2usize;
        bytes.extend_from_slice(&CHUNK_DATA.to_le_bytes());
        bytes.extend_from_slice(&((frame_size * frames) as u32).to_le_bytes());
        for frame in 0..frames {
            bytes.extend_from_slice(&[0x00, 0x00, 0x00, 0xf0, frame as u8, 0x78, 0x5a, 0x00]);
            bytes.extend(std::iter::repeat_n(0xfc, frame_size - LOPUS_FRAME_HEADER));
        }
        bytes
    }

    #[test]
    fn lopus_splits_fixed_frames() {
        let clip = parse_lopus(&build_lopus()).expect("应当解析成功");
        match clip {
            Clip::OpusPackets {
                sample_rate,
                channels,
                stride,
                skip,
                packets,
                frames,
            } => {
                assert_eq!(sample_rate, 48000);
                assert_eq!(channels, 2);
                assert_eq!(stride, 240);
                assert_eq!(skip, 120);
                assert_eq!(frames, 1920);
                assert_eq!(packets.len(), 480);
                assert_eq!(packets[0], 0xfc);
            }
            _ => panic!("应当识别成 Opus"),
        }
    }

    #[test]
    fn lopus_rejects_bad_magic() {
        let mut bytes = build_lopus();
        bytes[0] = 0;
        assert!(parse_lopus(&bytes).is_err());
    }

    #[test]
    fn codec_sniffs_headers() {
        assert_eq!(Codec::sniff(b"HCA\0\x03\x00\x00\x60"), Codec::Hca);
        assert_eq!(
            Codec::sniff(&[0x01, 0x00, 0x00, 0x80, 0x18, 0x00, 0x00, 0x00]),
            Codec::Lopus
        );
        assert_eq!(Codec::from_wave_ext(".lopus"), Codec::Lopus);
        assert_eq!(Codec::from_wave_ext("hca"), Codec::Hca);
    }
}
