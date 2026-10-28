/**
 * `VideoFrame` → `ImageBitmap`，**同步**完成。
 *
 * 为什么必须同步：WebCodecs 的 `VideoDecoder` 输出帧是有配额的（硬解通常十几块
 * 输出缓冲）。如果像以前那样把 `VideoFrame` 攒进数组、等解码完再统一转位图，
 * 池子一旦耗尽解码器就停下来等释放缓冲 —— 表现正是「吐出 13 帧后 flush 永不返回」。
 * 所以这里只在 `output` 回调里同步 `drawImage` + `transferToImageBitmap`，
 * 调用方紧接着 `frame.close()`，一帧不囤。
 *
 * 走 Canvas2D 的 `drawImage`（`VideoFrame` 本身就是合法的 `CanvasImageSource`），
 * 而不是 `createImageBitmap(frame, { resizeWidth, … })`：缩放交给画布，
 * `transferToImageBitmap()` 同步返回，且是零拷贝。
 */

/**
 * 这张位图是不是「近乎全黑」。
 *
 * 用途：`PcSelect_*.usm` 这类选人短片是**从黑场淡入**的，第 0 帧真的是纯黑
 * （1×1 缩略取平均灰 = 0）。解码修好之后首屏依然是黑的，很容易被当成「还是坏的」，
 * 所以在第 0 帧上做一次判定，界面给一句说明而不是让人猜。
 *
 * 拿不到像素时一律返回 `false`：这句提示是锦上添花，不能因为它出问题而挡路。
 */
export function isNearBlack(bitmap: ImageBitmap, threshold = 8): boolean {
  if (typeof OffscreenCanvas !== "function") return false;
  try {
    const probe = new OffscreenCanvas(1, 1);
    const context = probe.getContext("2d", { willReadFrequently: true });
    if (!context) return false;
    context.drawImage(bitmap, 0, 0, 1, 1);
    const [red, green, blue] = context.getImageData(0, 0, 1, 1).data;
    return red <= threshold && green <= threshold && blue <= threshold;
  } catch {
    return false;
  }
}

/** 一个 GOP 复用同一块画布：30 帧不会建 30 块。 */
export function createFrameBitmapConverter(
  width: number,
  height: number,
): (frame: VideoFrame) => ImageBitmap {
  const targetWidth = Math.max(1, Math.round(width));
  const targetHeight = Math.max(1, Math.round(height));
  let canvas: OffscreenCanvas | null = null;
  let context: OffscreenCanvasRenderingContext2D | null = null;

  return (frame: VideoFrame): ImageBitmap => {
    if (!canvas || !context) {
      if (typeof OffscreenCanvas !== "function") {
        throw new Error("当前 WebView 没有 OffscreenCanvas，无法把 VideoFrame 转成画面");
      }
      canvas = new OffscreenCanvas(targetWidth, targetHeight);
      const created = canvas.getContext("2d");
      if (!created) throw new Error("无法取得 OffscreenCanvas 的 2D 上下文");
      context = created;
    }
    context.clearRect(0, 0, targetWidth, targetHeight);
    context.drawImage(frame as unknown as CanvasImageSource, 0, 0, targetWidth, targetHeight);
    return canvas.transferToImageBitmap();
  };
}
