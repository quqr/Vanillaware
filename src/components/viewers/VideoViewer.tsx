import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, type FileEntry, type UsmVideo } from "../../lib/ipc";
import { decodeGop, describeError, UnsupportedVideoError, videoDecodable } from "../../lib/videoDecode";
import { createFrameBitmapConverter, isNearBlack } from "../../lib/frameBitmap";
import { gopIndexAt, gopRange, nextGopToLoad, type GopRange } from "../../lib/videoWindow";
import { mergeFrameBatch, type FrameBatchItem } from "../../lib/frameCache";

const TIMELINE_HEIGHT = 56;
/** 解码后转成位图时的最大宽度：够看清，又不至于把显存吃光。 */
const MAX_BITMAP_WIDTH = 1024;
/**
 * 提前多少帧开始取下一段 GOP。
 *
 * 取流 + 解码一次约 100~200 ms，30 fps 下就是 3~6 帧；留 12 帧（约 0.4 s）余量，
 * 播放到 GOP 边界时下一段已经在手里了。以前是**用完才取**，跨边界必然空一截，
 * 画布被清空 → 看到的就是「闪一下」。
 */
const PREFETCH_AHEAD = 12;

/** 跨进程取码流也要有上限：卡住就报错，不要让界面停在「取码流」。 */
function withDeadline<T>(promise: Promise<T>, ms: number, label: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => reject(new Error(`${label}超时（${ms} ms 无响应）`)), ms);
    promise.then(
      (value) => {
        window.clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        window.clearTimeout(timer);
        reject(error);
      },
    );
  });
}

/** USM 影片：画面 + 码率时间轴 + 逐帧控制。 */
export function VideoViewer({ entry }: { entry: FileEntry }) {
  const [info, setInfo] = useState<UsmVideo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<string>("建立索引…");

  /** 绝对帧号 → 位图。按帧号存，跨 GOP 才能无缝衔接（数组下标一旦换段就对不上）。 */
  const [frames, setFrames] = useState<Map<number, ImageBitmap>>(() => new Map());
  const [loaded, setLoaded] = useState<GopRange | null>(null);
  const [gopStats, setGopStats] = useState<{ gop: number; chunks: number; decoded: number } | null>(
    null,
  );
  const [current, setCurrent] = useState(0);
  const [playing, setPlaying] = useState(false);
  /** 非故障说明（例如「第 0 帧本来就是黑场」），放在底栏，不用警告色。 */
  const [hint, setHint] = useState("");

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const timelineRef = useRef<HTMLCanvasElement>(null);
  /** 位图表的同步真身：合并要读「上一次的结果」，不能等 React 的延后 updater。 */
  const framesRef = useRef<Map<number, ImageBitmap>>(new Map());
  const loadingRef = useRef(false);
  const generationRef = useRef(0);
  /** 播放到这里就停（`info.frames` 的镜像，避免 effect 里读不到最新值）。 */
  const framesTotalRef = useRef(0);
  /** 播放中不刷新阶段文案，否则顶部小黄标每秒闪一次。 */
  const playingRef = useRef(false);
  /** 播放定时器不依赖 `current`，用 ref 取当前值。 */
  const currentRef = useRef(0);

  // ─── 索引 ───────────────────────────────────────

  useEffect(() => {
    let alive = true;
    setInfo(null);
    setError(null);
    setStatus("建立索引…");
    framesRef.current.forEach((bitmap) => bitmap.close());
    framesRef.current = new Map();
    setFrames(framesRef.current);
    setLoaded(null);
    setGopStats(null);
    setHint("");
    setCurrent(0);
    setPlaying(false);

    api
      .usmVideoInfo(entry.path)
      .then((next) => {
        if (!alive) return;
        framesTotalRef.current = next.frames;
        setInfo(next);
        setStatus("");
      })
      .catch((e) => {
        if (alive) {
          setError(String(e));
          setStatus("");
        }
      });
    return () => {
      alive = false;
    };
  }, [entry.path]);

  const gopOf = useCallback(
    (frame: number) => gopIndexAt(info?.keyframes ?? [], frame),
    [info],
  );

  const loadGop = useCallback(
    async (gop: number) => {
      if (!info || loadingRef.current) return;
      loadingRef.current = true;
      // 代际令牌：重试时旧请求的迟到回调不再改状态。
      const generation = (generationRef.current += 1);
      const mine = () => generation === generationRef.current;
      const report = (text: string) => {
        if (!mine()) return;
        // 播放中后台预取很频繁，阶段文案会让顶部小黄标一直闪，跳过。
        if (playingRef.current) return;
        setStatus(text);
      };

      const range = gopRange(info.keyframes, info.frames, gop);
      const startFrame = range.start;
      const span = range.end - range.start;
      // `batch` 从声明到交给 state 都不再重新赋值：`setFrames` 的 updater 是**延后执行**的，
      // 如果像以前那样在 finally 里写 `fresh = []`，updater 读到的就是空数组
      // —— 表现正是「解出 30 帧、缓存 0 帧、画面全黑」。
      const batch: FrameBatchItem[] = [];
      let handedOver = false;

      report("取码流…");
      try {
        const bytes = await withDeadline(api.usmVideoGop(entry.path, gop), 15_000, "取码流");
        if (!mine()) return;
        const targetWidth = Math.min(MAX_BITMAP_WIDTH, info.width) || info.width || 640;
        const targetHeight = Math.max(
          2,
          Math.round((targetWidth * info.height) / Math.max(1, info.width)),
        );
        // 位图在解码器的 output 回调里**同步**生成：帧当场用完当场还给解码器，
        // 攒着不还会把硬解的输出缓冲耗尽（表现为只解出十几帧就卡死）。
        const toBitmap = createFrameBitmapConverter(targetWidth, targetHeight);
        const result = await decodeGop(
          new Uint8Array(bytes),
          info,
          startFrame,
          (frame) => {
            // 解码器会把我们给的 timestamp 原样送回来，据此定位绝对帧号。
            const index = Math.round((frame.timestamp * info.fps) / 1_000_000);
            batch.push({ index, bitmap: toBitmap(frame) });
          },
          report,
        );
        if (!mine()) return;

        setGopStats({ gop, chunks: result.chunks, decoded: result.decoded });
        // 解出的帧数与真正落进缓存的张数必须一致：不一致就说明转换那一步丢了帧，
        // 这种偏差不会抛错，只会在界面上表现为「黑屏 / 少一截」，所以要显式报出来。
        const lost =
          result.decoded > batch.length
            ? `解出 ${result.decoded} 帧但只转出 ${batch.length} 张位图`
            : "";
        const keepFrom = Math.max(0, startFrame - span);
        // 同步合并：读 ref 里的真身，不依赖 React 的延后 updater
        // （updater 里读闭包变量曾是「解出 30 帧、缓存 0 帧」的根因）。
        const merged = mergeFrameBatch(framesRef.current, batch, keepFrom);
        framesRef.current = merged.next;
        setFrames(merged.next);
        for (const bitmap of merged.dropped) bitmap.close();
        handedOver = true;
        setLoaded(range);
        setStatus(result.warning ?? lost);
        // 首段第 1 帧就是黑的，多半是影片本身从黑场淡入（PcSelect_* 全是），
        // 说明一句，免得把「正确解码出来的黑帧」当成又一次失败。
        if (gop === 0) {
          const first = batch.find((item) => item.index === startFrame) ?? batch[0];
          setHint(first && isNearBlack(first.bitmap) ? "第 1 帧是黑场（影片淡入），往后拖有画面" : "");
        }
      } catch (e) {
        report(e instanceof UnsupportedVideoError ? e.message : describeError(e));
      } finally {
        if (!handedOver) batch.forEach((item) => item.bitmap.close());
        loadingRef.current = false;
      }
    },
    [entry.path, info],
  );

  // 首屏先解第 0 段。
  useEffect(() => {
    if (!info || loaded) return;
    void loadGop(gopOf(current));
  }, [info, loaded, current, gopOf, loadGop]);

  // 需要哪一段：判断逻辑在 `videoWindow.ts`（纯函数，可单测）。
  useEffect(() => {
    if (!info || loadingRef.current) return;
    const target = nextGopToLoad({
      keyframes: info.keyframes,
      total: info.frames,
      current,
      loaded,
      has: (frame) => frames.has(frame),
      prefetchAhead: PREFETCH_AHEAD,
    });
    if (target !== null) void loadGop(target);
  }, [current, frames, info, loaded, loadGop]);

  // 卸载时释放位图：只动 ref，不 setState —— 在 StrictMode 的「模拟卸载再挂载」里
  // 清 state 会把已经解好的画面清空。
  useEffect(
    () => () => {
      framesRef.current.forEach((bitmap) => bitmap.close());
      framesRef.current = new Map();
    },
    [],
  );

  // ─── 播放 ───────────────────────────────────────

  useEffect(() => {
    playingRef.current = playing;
  }, [playing]);

  useEffect(() => {
    currentRef.current = current;
  }, [current]);

  // 按**墙钟**推进：解码会占住主线程几十到上百毫秒，`prev + 1` 的写法会把积压的
  // 定时器回调一次放出来，表现为「顿一下然后连跳几帧」。
  useEffect(() => {
    if (!playing || !info) return;
    const from = currentRef.current;
    const startedAt = performance.now();
    const fps = Math.max(1, info.fps);
    const interval = window.setInterval(() => {
      const elapsed = (performance.now() - startedAt) / 1000;
      const next = from + Math.floor(elapsed * fps);
      if (next >= framesTotalRef.current) {
        setCurrent(Math.max(0, framesTotalRef.current - 1));
        setPlaying(false);
        return;
      }
      setCurrent(next);
    }, Math.max(8, 500 / fps));
    return () => window.clearInterval(interval);
  }, [playing, info]);

  // ─── 画面 ───────────────────────────────────────

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const bitmap = frames.get(current);
    // 缺帧时**不清屏**：清屏就是那道黑闪。保持上一张，等下一段到了自然接上。
    if (!bitmap) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    if (canvas.width !== bitmap.width || canvas.height !== bitmap.height) {
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
    }
    context.drawImage(bitmap, 0, 0);
  }, [frames, current]);

  // ─── 时间轴 ─────────────────────────────────────

  useEffect(() => {
    const canvas = timelineRef.current;
    if (!canvas || !info) return;
    const paint = () => {
      const context = canvas.getContext("2d");
      if (!context) return;
      const rect = canvas.getBoundingClientRect();
      const width = Math.max(1, Math.floor(rect.width));
      const height = TIMELINE_HEIGHT;
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.floor(width * dpr);
      canvas.height = Math.floor(height * dpr);
      context.setTransform(dpr, 0, 0, dpr, 0, 0);
      context.clearRect(0, 0, width, height);

      const styles = getComputedStyle(canvas);
      const accent = styles.getPropertyValue("--color-primary").trim() || "#3b82f6";
      const faint = styles.getPropertyValue("--color-base-content").trim() || "#888";

      const sizes = info.frameSizes;
      let peak = 1;
      for (const size of sizes) if (size > peak) peak = size;
      const columns = Math.min(sizes.length, width);
      const step = sizes.length / columns;

      for (let column = 0; column < columns; column += 1) {
        const from = Math.floor(column * step);
        const to = Math.max(from + 1, Math.floor((column + 1) * step));
        let max = 0;
        let key = false;
        for (let index = from; index < to && index < sizes.length; index += 1) {
          if (sizes[index] > max) max = sizes[index];
          if (info.keyframes.includes(index)) key = true;
        }
        const barHeight = Math.max(1, (max / peak) * (height - 8));
        context.fillStyle = key ? accent : faint;
        context.globalAlpha = key ? 0.95 : 0.35;
        context.fillRect(column, height - barHeight, 1, barHeight);
      }
      context.globalAlpha = 1;

      // 关键帧刻度
      context.fillStyle = accent;
      for (const key of info.keyframes) {
        const x = (key / Math.max(1, sizes.length)) * width;
        context.fillRect(x, 0, 1, 4);
      }
    };
    paint();
    const observer = new ResizeObserver(paint);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [info]);

  const seek = useCallback(
    (frame: number) => {
      if (!info) return;
      const clamped = Math.max(0, Math.min(info.frames - 1, Math.round(frame)));
      setPlaying(false);
      setCurrent(clamped);
    },
    [info],
  );

  const onTimelineClick = (event: React.MouseEvent<HTMLDivElement>) => {
    if (!info) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (event.clientX - rect.left) / Math.max(1, rect.width)));
    seek(ratio * (info.frames - 1));
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) return;
      if (event.code === "Space") {
        event.preventDefault();
        setPlaying((value) => !value);
      } else if (event.code === "ArrowRight") {
        event.preventDefault();
        seek(current + 1);
      } else if (event.code === "ArrowLeft") {
        event.preventDefault();
        seek(current - 1);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [current, seek]);

  const progress = useMemo(() => {
    if (!info || !info.frames) return 0;
    return current / info.frames;
  }, [current, info]);

  if (error) {
    return (
      <div className="flex flex-1 items-center justify-center p-4 text-sm text-error">{error}</div>
    );
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5 text-[10px]">
        {info ? (
          <>
            <span className="badge badge-xs badge-primary">{info.codec}</span>
            <span className="badge badge-xs">
              {info.width}×{info.height}
            </span>
            <span className="badge badge-xs">{info.fps.toFixed(2)} fps</span>
            <span className="badge badge-xs">{formatTime(info.durationMs / 1000)}</span>
            <span className="badge badge-xs badge-ghost">{info.frames} 帧</span>
            <span className="badge badge-xs badge-ghost">{info.keyframes.length} 关键帧</span>
            <span className="badge badge-xs badge-ghost">音频 {info.audio}</span>
            <span className="badge badge-xs badge-ghost">
              {(info.streamBytes / (1024 * 1024)).toFixed(1)} MB
            </span>
          </>
        ) : (
          <span className="text-base-content/50">{status || "读取中…"}</span>
        )}
        {status && info && <span className="badge badge-xs badge-warning">{status}</span>}
        {!videoDecodable() && (
          <span className="badge badge-xs badge-warning">WebView 无 WebCodecs，无法出画面</span>
        )}
      </div>

      <div className="relative flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-base-300/40 p-2">
        <canvas ref={canvasRef} className="max-h-full max-w-full object-contain shadow" />
        {frames.size === 0 && (
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-2 p-6 text-center">
            <span
              className={`max-w-full text-xs ${status ? "text-warning" : "text-base-content/50"}`}
            >
              {status || (info ? "没有解码出画面" : "")}
            </span>
            {status && info && (
              <button
                className="btn btn-xs"
                onClick={() => {
                  // 上一次可能还卡在某个 await 上，放开门闩再重来。
                  loadingRef.current = false;
                  void loadGop(gopOf(current));
                }}
              >
                重试
              </button>
            )}
          </div>
        )}
      </div>

      <div className="flex flex-none flex-col gap-1 border-t border-base-300 p-2">
        <div
          className="relative w-full cursor-pointer overflow-hidden rounded border border-base-300 bg-base-100"
          style={{ height: TIMELINE_HEIGHT }}
          onClick={onTimelineClick}
        >
          <canvas ref={timelineRef} className="block h-full w-full" />
          <div
            className="pointer-events-none absolute inset-y-0 w-px bg-error"
            style={{ left: `${progress * 100}%` }}
          />
        </div>

        <div className="flex items-center gap-2">
          <button
            className="btn btn-sm btn-primary"
            disabled={!info}
            onClick={() => setPlaying((value) => !value)}
            title="空格"
          >
            {playing ? "暂停" : "播放"}
          </button>
          <button className="btn btn-sm" disabled={!info} onClick={() => seek(current - 1)}>
            上一帧
          </button>
          <button className="btn btn-sm" disabled={!info} onClick={() => seek(current + 1)}>
            下一帧
          </button>
          <button className="btn btn-sm" disabled={!info} onClick={() => seek(0)}>
            回到开头
          </button>
          <span className="font-mono text-[11px] text-base-content/60">
            {current + 1} / {info?.frames ?? 0} · {formatTime(current / Math.max(1, info?.fps ?? 30))}
          </span>
          {loaded && (
            <span className="ml-auto font-mono text-[10px] text-base-content/40">
              GOP {loaded.gop} · {loaded.start}..{loaded.end - 1}
              {gopStats && gopStats.gop === loaded.gop
                ? ` · 送包 ${gopStats.chunks} · 解出 ${gopStats.decoded}`
                : ""}
              {` · 缓存 ${frames.size} 帧`}
            </span>
          )}
          {hint && <span className="text-[10px] text-base-content/40">{hint}</span>}
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
