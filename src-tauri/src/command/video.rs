use tauri::State;
use tauri::ipc::Response;

use crate::model::video::UsmVideoDto;
use crate::state::AppState;

#[tauri::command]
pub async fn usm_video_info(
    state: State<'_, AppState>,
    path: String,
) -> Result<UsmVideoDto, String> {
    let target = super::real_path(&state, &path)?;
    let service = state.video.clone();

    tokio::task::spawn_blocking(move || {
        let index = service.open(&target)?;
        let fps = index.fps_centi as f32 / 100.0;
        let frames = index.frames.len() as u32;
        Ok(UsmVideoDto {
            width: index.width,
            height: index.height,
            codec: index.codec.clone(),
            profile: index.profile as u32,
            level: index.level as u32,
            fps,
            frames,
            duration_ms: if fps > 0.0 {
                (frames as f64 / fps as f64 * 1000.0) as u64
            } else {
                0
            },
            sps: hex(&index.sps),
            pps: hex(&index.pps),
            audio: index.audio.clone(),
            chunks: index.chunks.len() as u32,
            stream_bytes: index.stream_len,
            keyframes: index
                .keyframes()
                .into_iter()
                .map(|value| value as u32)
                .collect(),
            frame_sizes: index.frames.iter().map(|frame| frame.size).collect(),
        })
    })
    .await
    .map_err(|error| format!("join error: {error}"))?
}

#[tauri::command]
pub async fn usm_video_gop(
    state: State<'_, AppState>,
    path: String,
    gop: u32,
) -> Result<Response, String> {
    let target = super::real_path(&state, &path)?;
    let service = state.video.clone();

    let bytes = tokio::task::spawn_blocking(move || {
        let index = service.open(&target)?;
        let keys = index.keyframes();
        let position = gop as usize;
        let start_frame = *keys
            .get(position)
            .ok_or_else(|| format!("GOP {gop} out of range ({} total)", keys.len()))?;
        let end_frame = keys
            .get(position + 1)
            .copied()
            .unwrap_or(index.frames.len());
        let from = index.frames[start_frame].stream_offset;
        let to = index
            .frames
            .get(end_frame)
            .map(|frame| frame.stream_offset)
            .unwrap_or(index.stream_len);
        index
            .read_range(&target, from, to.saturating_sub(from))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("join error: {error}"))??;

    Ok(Response::new(bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
