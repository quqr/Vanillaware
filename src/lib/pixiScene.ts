/** PixiJS 渲染层：一个 Application + 一个相机 Container + mesh 池。
 *
 * 数据来源仍是 `Scene`（Rust 侧的预览场景快照）；这里只负责把「一帧的图层」画出来：
 * - 每个图层一个 `Mesh`，`MeshGeometry` 用 4 个顶点 + 2 个三角形；
 * - UV 用**归一化 0..1**（`srcquad` 原样）。Pixi 的两条默认路径都按归一化采样：
 *   内置 mesh 着色器 `uv = (uTextureMatrix * vec3(uv,1)).xy`；4 顶点的 mesh 走 batch
 *   时 UV 被原样写进顶点缓冲。传纹理像素坐标会让采样全部越界 clamp 到贴图边缘；
 * - 相机缩放/平移作用在父 Container 上，图层顶点保持世界坐标；
 * - 播放推进由 `app.ticker` 驱动，React 只接收节流后的帧号。
 */

import { Application, Container, Graphics, Mesh, MeshGeometry, Texture } from "pixi.js";

import type { Camera } from "./sceneDraw";

/** 一帧要提交给渲染器的图层。 */
export interface RenderLayer {
  /** 8 个世界坐标：dstquad 四角经 s7 矩阵映射后的结果。 */
  positions: Float32Array;
  /** 8 个归一化 UV（0..1）：srcquad 原样。 */
  uvs: Float32Array;
  texture: Texture;
  /** RGB 乘色（`fogquad × 时间线 color`）。 */
  tint: number;
  /** 乘色后的 alpha（同上，逐分量相乘的结果）。 */
  alpha: number;
}

/** 一帧的完整输入。 */
export interface RenderFrame {
  layers: RenderLayer[];
  /** 每个命中框 8 个世界坐标。 */
  hitboxes: Float32Array[];
}

/** mesh 池上限：超过就不再新建（多出的图层不画，避免异常数据把显存吃穿）。 */
const MAX_MESHES = 1024;

/** 一个可复用的四边形 mesh。 */
class QuadMesh {
  readonly mesh: Mesh<MeshGeometry>;
  private readonly positions: Float32Array;
  private readonly uvs: Float32Array;

  constructor() {
    this.positions = new Float32Array(8);
    this.uvs = new Float32Array(8);
    const geometry = new MeshGeometry({
      positions: this.positions,
      uvs: this.uvs,
      indices: new Uint32Array([0, 1, 2, 0, 2, 3]),
    });
    this.mesh = new Mesh({ geometry, texture: Texture.WHITE });
    this.mesh.visible = false;
  }

  update(layer: RenderLayer) {
    this.positions.set(layer.positions);
    this.uvs.set(layer.uvs);
    const positionBuffer = this.mesh.geometry.getBuffer("aPosition");
    positionBuffer.data.set(this.positions);
    positionBuffer.update();
    const uvBuffer = this.mesh.geometry.getBuffer("aUV");
    uvBuffer.data.set(this.uvs);
    uvBuffer.update();

    this.mesh.texture = layer.texture;
    this.mesh.tint = layer.tint;
    this.mesh.alpha = layer.alpha;
    this.mesh.visible = true;
  }

  hide() {
    this.mesh.visible = false;
  }
}

/** 提交一帧时的调试开关。 */
export interface SubmitOptions {
  showHitbox: boolean;
  showOutline: boolean;
}

/** 预览渲染器。生命周期与画布一致：`create` → `submit/setCamera/resize` → `destroy`。 */
export class PixiRenderer {
  private readonly app: Application;
  private readonly world: Container;
  private readonly quadLayer: Container;
  private readonly hitboxLayer: Graphics;
  private readonly outlineLayer: Graphics;
  private readonly originLayer: Graphics;
  private readonly pool: QuadMesh[] = [];
  private camera: Camera = { zoom: 1, panX: 0, panY: 0 };
  private width = 0;
  private height = 0;

  private constructor(app: Application) {
    this.app = app;
    this.world = new Container();
    this.quadLayer = new Container();
    this.hitboxLayer = new Graphics();
    this.outlineLayer = new Graphics();
    this.originLayer = new Graphics();

    this.world.addChild(this.quadLayer, this.hitboxLayer, this.outlineLayer, this.originLayer);
    app.stage.addChild(this.world);
  }

  /** 创建并初始化 Pixi 应用。画布由 Pixi 自己创建并挂到 `container`，避免 StrictMode 下复用同一个 canvas。 */
  static async create(
    container: HTMLElement,
    width: number,
    height: number,
    onTick: (deltaMs: number) => void,
  ): Promise<PixiRenderer> {
    const app = new Application();
    await app.init({
      width: Math.max(1, width),
      height: Math.max(1, height),
      backgroundAlpha: 0,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
      preference: "webgl",
    });
    container.appendChild(app.canvas);

    const renderer = new PixiRenderer(app);
    renderer.width = Math.max(1, width);
    renderer.height = Math.max(1, height);
    renderer.applyCamera();
    renderer.drawOrigin();

    app.ticker.add(() => onTick(app.ticker.deltaMS));
    return renderer;
  }

  /** Pixi 创建的画布（事件监听要挂在它上面）。 */
  get canvas(): HTMLCanvasElement {
    return this.app.canvas as HTMLCanvasElement;
  }

  /** 画布尺寸变化（CSS 像素）。 */
  resize(width: number, height: number) {
    this.width = Math.max(1, width);
    this.height = Math.max(1, height);
    this.app.renderer.resize(this.width, this.height);
    this.applyCamera();
    this.drawOrigin();
  }

  setCamera(camera: Camera) {
    this.camera = camera;
    this.applyCamera();
    this.drawOrigin();
  }

  /** 相机平移（拖拽）。 */
  panBy(dx: number, dy: number) {
    this.camera = { ...this.camera, panX: this.camera.panX + dx, panY: this.camera.panY + dy };
    this.applyCamera();
    this.drawOrigin();
  }

  /** 以画布中心为锚点缩放。 */
  zoomAt(factor: number) {
    const zoom = Math.min(64, Math.max(0.02, this.camera.zoom * factor));
    const ratio = zoom / this.camera.zoom;
    this.camera = {
      zoom,
      panX: this.camera.panX * ratio,
      panY: this.camera.panY * ratio,
    };
    this.applyCamera();
    this.drawOrigin();
  }

  getCamera(): Camera {
    return this.camera;
  }

  /** 提交一帧：复用池中的 mesh，多余的隐藏。 */
  submit(frame: RenderFrame, options: SubmitOptions) {
    const count = Math.min(frame.layers.length, MAX_MESHES);
    while (this.pool.length < count) {
      const quad = new QuadMesh();
      this.pool.push(quad);
      this.quadLayer.addChild(quad.mesh);
    }

    for (let index = 0; index < this.pool.length; index += 1) {
      if (index < count) {
        this.pool[index].update(frame.layers[index]);
      } else {
        this.pool[index].hide();
      }
    }

    this.drawHitboxes(options.showHitbox ? frame.hitboxes : []);
    this.drawOutlines(options.showOutline ? frame.layers : []);
  }

  destroy() {
    this.pool.length = 0;
    // 子节点（mesh 与 Graphics）由 Pixi 一并销毁；纹理由调用方按 texId 统一释放。
    this.app.destroy(true, { children: true, texture: false });
  }

  private applyCamera() {
    this.world.position.set(this.width / 2 + this.camera.panX, this.height / 2 + this.camera.panY);
    this.world.scale.set(this.camera.zoom);
  }

  /** 命中框：跟随相机缩放，线宽保持 1.5 个屏幕像素。 */
  private drawHitboxes(quads: Float32Array[]) {
    const gfx = this.hitboxLayer;
    gfx.clear();
    if (quads.length === 0) return;

    const width = 1.5 / this.camera.zoom;
    for (const quad of quads) {
      if (quad.length < 8) continue;
      gfx
        .poly([quad[0], quad[1], quad[2], quad[3], quad[4], quad[5], quad[6], quad[7]])
        .stroke({ width, color: 0xe4695f, alpha: 0.9 });
    }
  }

  /** 四边形描边：线宽随相机缩放保持 1 个屏幕像素。 */
  private drawOutlines(layers: RenderLayer[]) {
    const gfx = this.outlineLayer;
    gfx.clear();
    if (layers.length === 0) return;

    const width = 1 / this.camera.zoom;
    for (const layer of layers) {
      const p = layer.positions;
      gfx.poly([p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7]]).stroke({
        width,
        color: 0x3fd6a8,
        alpha: 0.85,
      });
    }
  }

  /** 原点十字：固定 12 个屏幕像素的臂长，缩放到 1 像素宽。 */
  private drawOrigin() {
    const gfx = this.originLayer;
    gfx.clear();
    const arm = 12 / this.camera.zoom;
    const width = 1 / this.camera.zoom;
    gfx
      .moveTo(-arm, 0)
      .lineTo(arm, 0)
      .moveTo(0, -arm)
      .lineTo(0, arm)
      .stroke({ width, color: 0x65737f, alpha: 0.7 });
  }
}

/** 一个 `#rrggbbaa` 颜色拆成 Pixi 的 tint 与 alpha。 */
export interface Tint {
  tint: number;
  alpha: number;
}

/** 白色不透明（乘法单位元）。 */
export const NO_TINT: Tint = { tint: 0xffffff, alpha: 1 };

/** `#rrggbbaa` → tint/alpha；缺省或非法视为白色不透明。 */
export function parseColor(color: string | null): Tint {
  if (!color || color.length !== 9 || !color.startsWith("#")) return NO_TINT;
  const value = Number.parseInt(color.slice(1), 16);
  if (Number.isNaN(value)) return NO_TINT;
  return { tint: (value >>> 8) & 0xffffff, alpha: (value & 0xff) / 255 };
}

/** 逐分量相乘（含 alpha），对应权威播放器的 `fog_multi4(color, fogquad)`。 */
export function multiplyTint(a: Tint, b: Tint): Tint {
  const ar = (a.tint >> 16) & 0xff;
  const ag = (a.tint >> 8) & 0xff;
  const ab = a.tint & 0xff;
  const br = (b.tint >> 16) & 0xff;
  const bg = (b.tint >> 8) & 0xff;
  const bb = b.tint & 0xff;
  return {
    tint: (Math.round((ar * br) / 255) << 16) | (Math.round((ag * bg) / 255) << 8) | Math.round((ab * bb) / 255),
    alpha: a.alpha * b.alpha,
  };
}

/** PNG 字节 → Pixi 纹理（ImageBitmap 解码一次，后续复用）。 */
export async function textureFromPng(bytes: ArrayBuffer): Promise<Texture> {
  const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
  return Texture.from(bitmap);
}
