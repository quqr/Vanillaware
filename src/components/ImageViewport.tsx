import { useCallback, useEffect, useRef, useState } from "react";

import { t } from "../i18n";

const CHECKER =
  "repeating-conic-gradient(color-mix(in oklab, currentColor 14%, transparent) 0% 25%, transparent 0% 50%)";

const MIN_ZOOM = 0.02;
const MAX_ZOOM = 64;
const FIT_MARGIN = 0.98;
const FIT_MAX = 4;
const WHEEL_STEP = 1.12;

interface Camera {
  zoom: number;
  panX: number;
  panY: number;
}

export interface ImageViewportProps {
  src: string;
  alt: string;
  className?: string;
  checker?: boolean;
  fitPadding?: number;
}

export function ImageViewport({
  src,
  alt,
  className = "",
  checker = true,
  fitPadding = 8,
}: ImageViewportProps) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const imageRef = useRef<HTMLImageElement>(null);
  const cameraRef = useRef<Camera>({ zoom: 1, panX: 0, panY: 0 });
  const touchedRef = useRef(false);
  const percentRef = useRef(100);
  const [percent, setPercent] = useState(100);

  const paint = useCallback(() => {
    const image = imageRef.current;
    if (!image) return;
    const { zoom, panX, panY } = cameraRef.current;
    image.style.transform = `translate(${panX}px, ${panY}px) scale(${zoom}) translate(-50%, -50%)`;
    image.style.imageRendering = zoom >= 1 ? "pixelated" : "auto";
    const next = Math.round(zoom * 100);
    if (next !== percentRef.current) {
      percentRef.current = next;
      setPercent(next);
    }
  }, []);

  const fit = useCallback(() => {
    const wrap = wrapRef.current;
    const image = imageRef.current;
    if (!wrap || !image) return;
    const width = wrap.clientWidth - fitPadding * 2;
    const height = wrap.clientHeight - fitPadding * 2;
    const sourceWidth = image.naturalWidth || 1;
    const sourceHeight = image.naturalHeight || 1;
    if (width <= 0 || height <= 0) return;
    const zoom = Math.min((width / sourceWidth) * FIT_MARGIN, (height / sourceHeight) * FIT_MARGIN, FIT_MAX);
    cameraRef.current = { zoom: clamp(zoom, MIN_ZOOM, MAX_ZOOM), panX: 0, panY: 0 };
    touchedRef.current = false;
    paint();
  }, [fitPadding, paint]);

  const zoomTo = useCallback(
    (target: number) => {
      const camera = cameraRef.current;
      const zoom = clamp(target, MIN_ZOOM, MAX_ZOOM);
      if (zoom === camera.zoom) return;
      const ratio = zoom / camera.zoom;
      cameraRef.current = { zoom, panX: camera.panX * ratio, panY: camera.panY * ratio };
      touchedRef.current = true;
      paint();
    },
    [paint],
  );

  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;

    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const factor = event.deltaY < 0 ? WHEEL_STEP : 1 / WHEEL_STEP;
      zoomTo(cameraRef.current.zoom * factor);
    };

    let dragging = false;
    let lastX = 0;
    let lastY = 0;
    let pointerId = -1;

    const onDown = (event: PointerEvent) => {
      if (event.button !== 0) return;
      dragging = true;
      lastX = event.clientX;
      lastY = event.clientY;
      pointerId = event.pointerId;
      wrap.setPointerCapture(event.pointerId);
      wrap.dataset.panning = "true";
    };

    const onMove = (event: PointerEvent) => {
      if (!dragging) return;
      const camera = cameraRef.current;
      cameraRef.current = {
        ...camera,
        panX: camera.panX + (event.clientX - lastX),
        panY: camera.panY + (event.clientY - lastY),
      };
      lastX = event.clientX;
      lastY = event.clientY;
      touchedRef.current = true;
      paint();
    };

    const onUp = () => {
      dragging = false;
      if (pointerId >= 0 && wrap.hasPointerCapture(pointerId)) wrap.releasePointerCapture(pointerId);
      pointerId = -1;
      wrap.dataset.panning = "false";
    };

    const onDoubleClick = () => {
      if (Math.abs(cameraRef.current.zoom - 1) < 1e-3) {
        fit();
        return;
      }
      zoomTo(1);
    };

    wrap.addEventListener("wheel", onWheel, { passive: false });
    wrap.addEventListener("dblclick", onDoubleClick);
    wrap.addEventListener("pointerdown", onDown);
    wrap.addEventListener("pointermove", onMove);
    wrap.addEventListener("pointerup", onUp);
    wrap.addEventListener("pointercancel", onUp);

    return () => {
      wrap.removeEventListener("wheel", onWheel);
      wrap.removeEventListener("dblclick", onDoubleClick);
      wrap.removeEventListener("pointerdown", onDown);
      wrap.removeEventListener("pointermove", onMove);
      wrap.removeEventListener("pointerup", onUp);
      wrap.removeEventListener("pointercancel", onUp);
    };
  }, [fit, paint, zoomTo]);

  useEffect(() => {
    paint();
  }, [paint]);

  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;
    const observer = new ResizeObserver(() => {
      if (!touchedRef.current) fit();
    });
    observer.observe(wrap);
    return () => observer.disconnect();
  }, [fit]);

  useEffect(() => {
    touchedRef.current = false;
    cameraRef.current = { zoom: 1, panX: 0, panY: 0 };
    const image = imageRef.current;
    if (image?.complete) fit();
  }, [fit, src]);

  return (
    <div
      className={`relative min-h-0 cursor-grab touch-none overflow-hidden active:cursor-grabbing ${className}`}
      ref={wrapRef}
      style={checker ? { backgroundImage: CHECKER, backgroundSize: "16px 16px" } : undefined}
    >
      <img
        ref={imageRef}
        className="absolute left-1/2 top-1/2 max-w-none select-none"
        style={{ transformOrigin: "0 0" }}
        src={src}
        alt={alt}
        draggable={false}
        onLoad={() => fit()}
      />

      <div className="pointer-events-none absolute left-2 top-2 flex items-center gap-1.5">
        <span className="badge badge-sm bg-base-100/90 font-mono">{percent}%</span>
        <button
          type="button"
          className="btn btn-xs pointer-events-auto bg-base-100/90"
          onClick={fit}
          title={t("image.fitHint")}
        >
          {t("image.fit")}
        </button>
      </div>
    </div>
  );
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}
