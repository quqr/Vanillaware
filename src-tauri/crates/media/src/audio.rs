use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use cridecoder::acb::{AfsArchive, TrackList, UtfTable, Value};
use cridecoder::HcaDecoder;

pub const LOPUS_FRAME_HEADER: usize = 8;

pub const CHUNK_BASIC: u32 = 0x8000_0001;
pub const CHUNK_DATA: u32 = 0x8000_0004;

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
            AudioError::Malformed(message) => write!(f, "corrupt container: {message}"),
            AudioError::NoTracks => write!(f, "the container has no playable track"),
            AudioError::OutOfRange(index) => write!(f, "track {index} out of range"),
            AudioError::Decode(message) => write!(f, "decode failed: {message}"),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<std::io::Error> for AudioError {
    fn from(error: std::io::Error) -> Self {
        AudioError::Io(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Codec {
    Hca,
    Lopus,
    Adx,
    Other(String),
}

impl Codec {
    pub fn tag(&self) -> String {
        match self {
            Codec::Hca => "HCA".to_string(),
            Codec::Lopus => "Opus".to_string(),
            Codec::Adx => "ADX".to_string(),
            Codec::Other(ext) => ext.to_uppercase(),
        }
    }

    pub fn playable(&self) -> bool {
        matches!(self, Codec::Hca | Codec::Lopus)
    }

    pub fn from_wave_ext(ext: &str) -> Self {
        match ext.to_ascii_lowercase().trim_start_matches('.') {
            "hca" => Codec::Hca,
            "lopus" | "opus" => Codec::Lopus,
            "adx" => Codec::Adx,
            other => Codec::Other(other.to_string()),
        }
    }

    pub fn sniff(bytes: &[u8]) -> Self {
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

#[derive(Clone, Debug)]
pub struct TrackInfo {
    pub index: usize,
    pub name: String,
    pub cue_id: i32,
    pub codec: Codec,
    pub stream: bool,
    pub stream_awb_id: i32,
    pub wav_id: i32,
    pub bytes: u64,
}

pub struct Bundle {
    pub source: PathBuf,
    pub container: &'static str,
    pub tracks: Vec<TrackInfo>,
    pub external: Vec<PathBuf>,
    pub warnings: Vec<String>,
    embedded_awb: Option<Vec<u8>>,
}

impl Bundle {
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
                        "track {} points at external AWB #{}, but only {} were found",
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
            .ok_or_else(|| AudioError::Malformed(format!("track {} has no embedded AWB", track.name)))?;
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
        warnings.push("the ACB has neither an embedded AWB nor a sibling streaming AWB".to_string());
    }

    let mut seen = std::collections::BTreeSet::new();
    for wave in &external {
        if !wave.exists() {
            warnings.push(format!("missing external AWB: {}", wave.display()));
        } else if !seen.insert(wave.clone()) {
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

pub fn open_awb(path: &Path) -> Result<Bundle, AudioError> {
    let archive = open_afs(path)?;
    if archive.files.is_empty() {
        return Err(AudioError::NoTracks);
    }

    let mut heads = File::open(path)?;
    let mut sniff = |offset: u32| -> Codec {
        let mut head = [0u8; 8];
        if heads.seek(SeekFrom::Start(u64::from(offset))).is_err() {
            return Codec::Other("?".to_string());
        }
        match heads.read(&mut head) {
            Ok(read) if read > 0 => Codec::sniff(&head[..read]),
            _ => Codec::Other("?".to_string()),
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
            codec: sniff(entry.offset),
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

pub enum Clip {
    Pcm {
        sample_rate: u32,
        channels: u32,
        frames: u64,
        samples: Vec<i16>,
    },
    OpusPackets {
        sample_rate: u32,
        channels: u32,
        frames: u64,
        stride: u32,
        skip: u32,
        packets: Vec<u8>,
    },
}

pub fn decode_track(bundle: &Bundle, index: usize) -> Result<Clip, AudioError> {
    let bytes = bundle.read_track(index)?;
    let declared = bundle
        .tracks
        .get(index)
        .map(|track| track.codec.clone())
        .unwrap_or_else(|| Codec::Other("?".to_string()));
    match effective_codec(&declared, &bytes) {
        Codec::Hca => decode_hca(&bytes),
        Codec::Lopus => parse_lopus(&bytes),
        other => Err(AudioError::Decode(format!(
            "{} cannot be decoded in the app yet",
            other.tag()
        ))),
    }
}

pub fn effective_codec(declared: &Codec, bytes: &[u8]) -> Codec {
    match Codec::sniff(bytes) {
        Codec::Other(_) => declared.clone(),
        sniffed => sniffed,
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

pub fn parse_lopus(bytes: &[u8]) -> Result<Clip, AudioError> {
    if bytes.len() < 0x20 {
        return Err(AudioError::Malformed("lopus header is shorter than 0x20 bytes".to_string()));
    }
    let Some(base) = (0..bytes.len().min(0x20)).find(|&at| u32_at(bytes, at) == CHUNK_BASIC) else {
        return Err(AudioError::Malformed(format!(
            "lopus is missing the basic chunk (read {:#x})",
            u32_at(bytes, 0)
        )));
    };
    let at = |offset: usize| base + offset;
    let channels = bytes[at(0x09)] as u32;
    let frame_size = u16_at(bytes, at(0x0a)) as usize;
    let sample_rate = u32_at(bytes, at(0x0c));
    let data_offset = u32_at(bytes, at(0x10)) as usize;
    let skip = u16_at(bytes, at(0x1c)) as u32;
    if frame_size <= LOPUS_FRAME_HEADER || sample_rate == 0 {
        return Err(AudioError::Malformed(format!(
            "invalid lopus parameters: frame_size={frame_size} sample_rate={sample_rate}"
        )));
    }
    let data_at = at(data_offset);
    if u32_at(bytes, data_at) != CHUNK_DATA {
        return Err(AudioError::Malformed(format!(
            "no data chunk at {data_at:#x}"
        )));
    }
    let data_size = u32_at(bytes, data_at + 4) as usize;
    let start = data_at + 8;
    let end = start.saturating_add(data_size).min(bytes.len());
    let body = &bytes[start.min(end)..end];
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
