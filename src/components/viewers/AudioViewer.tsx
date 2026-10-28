import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { api, type AudioBundle, type AudioClip, type FileEntry } from "../../lib/ipc";
import { decodeClip, buildPeaks, opusDecodable, UnsupportedAudioError, type DecodedAudio } from "../../lib/audioDecode";

const WAVE_HEIGHT = 132;

/** 音频容器：左侧 cue/条目列表，右侧波形与播放控制。 */
export function AudioViewer({ entry }: { entry: FileEntry }) {
  const [bundle, setBundle] = useState<AudioBundle | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<number | null>(null);

  const [clip, setClip] = useState<AudioClip | null>(null);
  const [decoded, setDecoded] = useState<DecodedAudio | null>(null);
  const [busy, setBusy] = useState(false);
  const [clipError, setClipError] = useState<string | null>(null);

  const [playing, setPlaying] = useState(false);
  const [position, setPosition] = useState(0);

  const listRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const waveRef = useRef<HTMLDivElement>(null);
  const ctxRef = useRef<AudioContext | null>(null);
  const sourceRef = useRef<AudioBufferSourceNode | null>(null);
  const startedAtRef = useRef(0);
  const rafRef = useRef(0);

  // ─── 容器索引 ───────────────────────────────────

  useEffect(() => {
    let alive = true;
    setBundle(null);
    setError(null);
    setSelected(null);
    setQuery("");
    api
      .audioBundle(entry.path)
      .then((next) => {
        if (!alive) return;
        setBundle(next);
        const first = next.tracks.find((track) => track.playable) ?? next.tracks[0];
        setSelected(first ? first.index : null);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      });
    return () => {
      alive = false;
    };
  }, [entry.path]);

  const stopPlayback = useCallback(() => {
    cancelAnimationFrame(rafRef.current);
    const source = sourceRef.current;
    sourceRef.current = null;
    if (source) {
      try {
        source.stop();
      } catch {
        // 已经结束的 source 再 stop 会抛，忽略。
      }
    }
    setPlaying(false);
  }, []);

  // ─── 片段解码 ───────────────────────────────────

  useEffect(() => {
    if (selected === null) return;
    let alive = true;
    stopPlayback();
    setPosition(0);
    setClip(null);
    setDecoded(null);
    setClipError(null);
    setBusy(true);

    (async () => {
      const info = await api.audioClipInfo(entry.path, selected);
      if (!alive) return;
      setClip(info);
      if (!info.stride && info.codec === "lopus") {
        throw new UnsupportedAudioError("lopus 缺少帧步长");
      }
      const data = await api.audioClipData(entry.path, selected);
      if (!alive) return;
      const next = await decodeClip(info, data);
      if (alive) setDecoded(next);
    })()
      .catch((e) => {
        if (alive) setClipError(e instanceof UnsupportedAudioError ? e.message : String(e?.message ?? e));
      })
      .finally(() => {
        if (alive) setBusy(false);
      });

    return () => {
      alive = false;
    };
  }, [entry.path, selected, stopPlayback]);

  useEffect(() => stopPlayback, [stopPlayback]);

  // ─── 播放 ───────────────────────────────────────

  const play = useCallback(
    (from: number) => {
      if (!decoded) return;
      stopPlayback();
      const ctx = ctxRef.current ?? new AudioContext();
      ctxRef.current = ctx;
      if (ctx.state === "suspended") void ctx.resume();
      const buffer = ctx.createBuffer(decoded.channels.length, decoded.frames, decoded.sampleRate);
      decoded.channels.forEach((plane, index) => buffer.copyToChannel(plane, index));
      const source = ctx.createBufferSource();
      source.buffer = buffer;
      source.connect(ctx.destination);
      source.onended = () => {
        if (sourceRef.current === source) {
          sourceRef.current = null;
          setPlaying(false);
        }
      };
      const offset = Math.min(Math.max(0, from), Math.max(0, decoded.durationSec - 0.01));
      source.start(0, offset);
      sourceRef.current = source;
      startedAtRef.current = ctx.currentTime - offset;
      setPlaying(true);

      const tick = () => {
        const audio = ctxRef.current;
        if (!audio || !sourceRef.current) return;
        const next = audio.currentTime - startedAtRef.current;
        setPosition(Math.min(decoded.durationSec, Math.max(0, next)));
        rafRef.current = requestAnimationFrame(tick);
      };
      rafRef.current = requestAnimationFrame(tick);
    },
    [decoded, stopPlayback],
  );

  const pause = useCallback(() => {
    stopPlayback();
  }, [stopPlayback]);

  const seek = useCallback(
    (seconds: number) => {
      setPosition(seconds);
      if (playing) {
        play(seconds);
      }
    },
    [playing, play],
  );

  const togglePlay = useCallback(() => {
    if (!decoded) return;
    if (playing) {
      pause();
    } else {
      play(position >= decoded.durationSec - 0.02 ? 0 : position);
    }
  }, [decoded, pause, play, playing, position]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) return;
      if (event.code === "Space") {
        event.preventDefault();
        togglePlay();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [togglePlay]);

  // ─── 波形绘制 ───────────────────────────────────

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const paint = () => {
      const context = canvas.getContext("2d");
      if (!context) return;
      const rect = canvas.getBoundingClientRect();
      const width = Math.max(1, Math.floor(rect.width));
      const height = WAVE_HEIGHT;
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      context.setTransform(dpr, 0, 0, dpr, 0, 0);
      context.clearRect(0, 0, width, height);

      const styles = getComputedStyle(canvas);
      const accent = styles.getPropertyValue("--color-primary").trim() || "#3b82f6";
      const faint = styles.getPropertyValue("--color-base-content").trim() || "#888";

      context.strokeStyle = faint;
      context.globalAlpha = 0.25;
      context.beginPath();
      context.moveTo(0, height / 2);
      context.lineTo(width, height / 2);
      context.stroke();
      context.globalAlpha = 1;

      if (!decoded) return;
      const columns = Math.floor(width);
      const peaks = buildPeaks(decoded.channels, columns);
      context.fillStyle = accent;
      const middle = height / 2;
      const scale = height / 2 - 4;
      for (let column = 0; column < columns; column += 1) {
        const min = peaks[column * 2];
        const max = peaks[column * 2 + 1];
        const top = middle - max * scale;
        const bottom = middle - min * scale;
        context.fillRect(column, top, 1, Math.max(1, bottom - top));
      }
    };
    paint();
    const observer = new ResizeObserver(paint);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [decoded]);

  const onWaveClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!decoded) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (event.clientX - rect.left) / Math.max(1, rect.width)));
    seek(ratio * decoded.durationSec);
  };

  // ─── 列表 ───────────────────────────────────────

  const tracks = useMemo(() => {
    if (!bundle) return [];
    const needle = query.trim().toLowerCase();
    if (!needle) return bundle.tracks;
    return bundle.tracks.filter(
      (track) =>
        track.name.toLowerCase().includes(needle) || String(track.cueId).includes(needle),
    );
  }, [bundle, query]);

  const virtualizer = useVirtualizer({
    count: tracks.length,
    getScrollElement: () => listRef.current,
    estimateSize: () => 26,
    overscan: 20,
  });

  if (error) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>
    );
  }
  if (!bundle) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 font-mono text-sm text-base-content/50">
        读取音频容器…
      </div>
    );
  }

  const progress = decoded ? Math.min(1, position / Math.max(0.001, decoded.durationSec)) : 0;

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <div className="flex w-64 flex-none flex-col border-r border-base-300">
        <div className="flex flex-none flex-col gap-1 border-b border-base-300 p-1.5">
          <input
            className="input input-xs w-full"
            placeholder="过滤 cue / id"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
          <div className="flex flex-wrap items-center gap-1 text-[10px]">
            <span className="badge badge-xs">{bundle.container.toUpperCase()}</span>
            <span className="badge badge-xs">{bundle.tracks.length} 条</span>
            {bundle.codecs.map((item) => (
              <span key={item.codec} className="badge badge-xs badge-ghost">
                {item.codec} × {item.count}
              </span>
            ))}
          </div>
        </div>

        <div ref={listRef} className="min-h-0 flex-1 overflow-auto">
          <div className="relative w-full" style={{ height: virtualizer.getTotalSize() }}>
            {virtualizer.getVirtualItems().map((row) => {
              const track = tracks[row.index];
              const active = track.index === selected;
              return (
                <button
                  key={track.index}
                  className={`absolute inset-x-0 flex items-center gap-1 px-2 text-left font-mono text-[11px] ${
                    active ? "bg-primary/15 text-primary" : "hover:bg-base-200"
                  }`}
                  style={{ height: row.size, transform: `translateY(${row.start}px)` }}
                  title={track.name}
                  onClick={() => setSelected(track.index)}
                >
                  <span className="w-10 flex-none text-right text-base-content/40">
                    {track.cueId}
                  </span>
                  <span className="flex-1 truncate">{track.name}</span>
                  <span
                    className={`badge badge-xs ${track.playable ? "badge-outline" : "badge-ghost opacity-60"}`}
                  >
                    {track.codec}
                  </span>
                </button>
              );
            })}
          </div>
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5 text-[10px]">
          {clip ? (
            <>
              <span className="badge badge-xs badge-primary">{clip.codec.toUpperCase()}</span>
              <span className="badge badge-xs">{clip.sampleRate} Hz</span>
              <span className="badge badge-xs">{clip.channels} ch</span>
              <span className="badge badge-xs">{formatTime(clip.durationMs / 1000)}</span>
              {clip.stride > 0 && <span className="badge badge-xs badge-ghost">包 {clip.stride} B</span>}
              {clip.skip > 0 && <span className="badge badge-xs badge-ghost">skip {clip.skip}</span>}
            </>
          ) : (
            <span className="text-base-content/50">未选择轨道</span>
          )}
          {busy && <span className="loading loading-spinner loading-xs" />}
          {!opusDecodable() && (
            <span className="badge badge-xs badge-warning">WebView 无 WebCodecs，Opus 不可播</span>
          )}
        </div>

        <div className="flex flex-none flex-col gap-1 p-2">
          <div
            ref={waveRef}
            className="relative w-full cursor-crosshair overflow-hidden rounded border border-base-300 bg-base-100"
            style={{ height: WAVE_HEIGHT }}
            onClick={onWaveClick}
          >
            <canvas ref={canvasRef} className="block h-full w-full" />
            {decoded && (
              <div
                className="pointer-events-none absolute inset-y-0 w-px bg-primary"
                style={{ left: `${progress * 100}%` }}
              />
            )}
            {!decoded && (
              <div className="absolute inset-0 flex items-center justify-center text-[11px] text-base-content/50">
                {busy ? "解码中…" : (clipError ?? "无波形")}
              </div>
            )}
          </div>

          <div className="flex items-center gap-2">
            <button
              className="btn btn-sm btn-primary"
              disabled={!decoded}
              onClick={togglePlay}
              title="空格"
            >
              {playing ? "暂停" : "播放"}
            </button>
            <button
              className="btn btn-sm"
              disabled={!decoded}
              onClick={() => {
                pause();
                setPosition(0);
              }}
            >
              回到开头
            </button>
            <span className="font-mono text-[11px] text-base-content/60">
              {formatTime(position)} / {formatTime(decoded?.durationSec ?? 0)}
            </span>
            {decoded && decoded.skipped > 0 && (
              <span className="font-mono text-[10px] text-base-content/40">
                已裁 {decoded.skipped} 前导样本
              </span>
            )}
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-auto border-t border-base-300 p-2 text-[11px]">
          {clipError && <div className="alert alert-warning alert-soft py-1 text-[11px]">{clipError}</div>}
          {bundle.warnings.map((warning) => (
            <div key={warning} className="text-warning">
              {warning}
            </div>
          ))}
          {bundle.external.length > 0 && (
            <details className="mt-1">
              <summary className="cursor-pointer text-base-content/50">
                外部 AWB（{bundle.external.length}）
              </summary>
              <ul className="mt-1 space-y-0.5 font-mono text-[10px] text-base-content/50">
                {bundle.external.map((path) => (
                  <li key={path} className="truncate" title={path}>
                    {path.split(/[\\/]/).pop()}
                  </li>
                ))}
              </ul>
            </details>
          )}
        </div>
      </div>
    </div>
  );
}

function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00.0";
  const minutes = Math.floor(seconds / 60);
  const rest = seconds - minutes * 60;
  return `${minutes}:${rest.toFixed(1).padStart(4, "0")}`;
}
