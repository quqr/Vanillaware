//! 音频命令：取容器索引、片段元信息、片段二进制。
//!
//! 分离三件事是为了不把几十兆 PCM 塞进 JSON：
//! `audio_clip_info` 只回元信息，`audio_clip_data` 用二进制响应回载荷。

use std::path::PathBuf;

use tauri::State;
use tauri::ipc::Response;

use media::audio::{Clip, decode_track};

use crate::model::audio::{AudioBundleDto, AudioClipDto, AudioCodecCountDto, AudioTrackDto};
use crate::state::AppState;

/// 打开 ACB/AWB，列出全部轨道。
#[tauri::command]
pub async fn audio_bundle(
    state: State<'_, AppState>,
    path: String,
) -> Result<AudioBundleDto, String> {
    let target = PathBuf::from(&path);
    let service = state.audio.clone();

    tokio::task::spawn_blocking(move || {
        let bundle = service.open(&target).map_err(|error| error.to_string())?;

        let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
        for track in &bundle.tracks {
            *counts.entry(track.codec.tag()).or_default() += 1;
        }
        let mut codecs: Vec<AudioCodecCountDto> = counts
            .into_iter()
            .map(|(codec, count)| AudioCodecCountDto { codec, count })
            .collect();
        codecs.sort_by(|a, b| b.count.cmp(&a.count).then(a.codec.cmp(&b.codec)));

        Ok(AudioBundleDto {
            container: bundle.container.to_string(),
            source: bundle.source.to_string_lossy().into_owned(),
            tracks: bundle
                .tracks
                .iter()
                .map(|track| AudioTrackDto {
                    index: track.index as u32,
                    name: track.name.clone(),
                    cue_id: track.cue_id,
                    codec: track.codec.tag(),
                    playable: track.codec.playable(),
                    stream: track.stream,
                    bytes: track.bytes,
                })
                .collect(),
            codecs,
            external: bundle
                .external
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            warnings: bundle.warnings.clone(),
        })
    })
    .await
    .map_err(|error| format!("join error: {error}"))?
}

/// 片段元信息 + 载荷长度。
#[tauri::command]
pub async fn audio_clip_info(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<AudioClipDto, String> {
    let target = PathBuf::from(&path);
    let service = state.audio.clone();

    tokio::task::spawn_blocking(move || {
        let bundle = service.open(&target).map_err(|error| error.to_string())?;
        let clip = decode_track(&bundle, index as usize).map_err(|error| error.to_string())?;
        Ok(describe(&clip))
    })
    .await
    .map_err(|error| format!("join error: {error}"))?
}

/// 片段载荷：HCA 为交错 i16 PCM，lopus 为定长裸 Opus 包拼接。
#[tauri::command]
pub async fn audio_clip_data(
    state: State<'_, AppState>,
    path: String,
    index: u32,
) -> Result<Response, String> {
    let target = PathBuf::from(&path);
    let service = state.audio.clone();

    let bytes = tokio::task::spawn_blocking(move || {
        let bundle = service.open(&target).map_err(|error| error.to_string())?;
        let clip = decode_track(&bundle, index as usize).map_err(|error| error.to_string())?;
        Ok::<Vec<u8>, String>(match clip {
            Clip::Pcm { samples, .. } => {
                let mut bytes = Vec::with_capacity(samples.len() * 2);
                for sample in samples {
                    bytes.extend_from_slice(&sample.to_le_bytes());
                }
                bytes
            }
            Clip::OpusPackets { packets, .. } => packets,
        })
    })
    .await
    .map_err(|error| format!("join error: {error}"))??;

    Ok(Response::new(bytes))
}

fn describe(clip: &Clip) -> AudioClipDto {
    match clip {
        Clip::Pcm {
            sample_rate,
            channels,
            frames,
            samples,
        } => AudioClipDto {
            codec: "hca".to_string(),
            sample_rate: *sample_rate,
            channels: *channels,
            frames: *frames,
            duration_ms: duration_ms(*frames, *sample_rate),
            stride: 0,
            skip: 0,
            bytes: (samples.len() * 2) as u64,
        },
        Clip::OpusPackets {
            sample_rate,
            channels,
            frames,
            stride,
            skip,
            packets,
        } => AudioClipDto {
            codec: "lopus".to_string(),
            sample_rate: *sample_rate,
            channels: *channels,
            frames: *frames,
            duration_ms: duration_ms(*frames, *sample_rate),
            stride: *stride,
            skip: *skip,
            bytes: packets.len() as u64,
        },
    }
}

fn duration_ms(frames: u64, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        return 0;
    }
    frames.saturating_mul(1000) / sample_rate as u64
}
