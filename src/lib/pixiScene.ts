import { Application, Container, Graphics, Mesh, MeshGeometry, Texture } from "pixi.js";

import type { Camera } from "./sceneDraw";

export interface RenderLayer {

  positions: Float32Array;

  uvs: Float32Array;
  texture: Texture;

  tint: number;

  alpha: number;
}

export interface RenderFrame {
  layers: RenderLayer[];

  hitboxes: Float32Array[];
}

const MAX_MESHES = 1024;

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

export interface SubmitOptions {
  showHitbox: boolean;
  showOutline: boolean;
}

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

  get canvas(): HTMLCanvasElement {
    return this.app.canvas as HTMLCanvasElement;
  }

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

  panBy(dx: number, dy: number) {
    this.camera = { ...this.camera, panX: this.camera.panX + dx, panY: this.camera.panY + dy };
    this.applyCamera();
    this.drawOrigin();
  }

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
    this.app.destroy(true, { children: true, texture: false });
  }

  private applyCamera() {
    this.world.position.set(this.width / 2 + this.camera.panX, this.height / 2 + this.camera.panY);
    this.world.scale.set(this.camera.zoom);
  }

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

export interface Tint {
  tint: number;
  alpha: number;
}

export const NO_TINT: Tint = { tint: 0xffffff, alpha: 1 };

export function parseColor(color: string | null): Tint {
  if (!color || color.length !== 9 || !color.startsWith("#")) return NO_TINT;
  const value = Number.parseInt(color.slice(1), 16);
  if (Number.isNaN(value)) return NO_TINT;
  return { tint: (value >>> 8) & 0xffffff, alpha: (value & 0xff) / 255 };
}

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

export async function textureFromPng(bytes: ArrayBuffer): Promise<Texture> {
  const bitmap = await createImageBitmap(new Blob([bytes], { type: "image/png" }));
  return Texture.from(bitmap);
}
