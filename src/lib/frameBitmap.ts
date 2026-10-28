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
