/** 可视化网格 → Three 对象：UV、按纹理页分组与贴图加载（FMD / OSB 共用）。 */

import type { Color, Mesh, MeshStandardMaterial, Texture } from "three";
import { api, type VizMesh } from "./ipc";

/** 解码 FTX 的指定页为贴图；缺页或解码失败时跳过，调用方退回纯色。 */
export async function loadPageTextures(
  THREE: typeof import("three"),
  ftxPath: string | null,
  pages: number[],
): Promise<Map<number, Texture>> {
  const textures = new Map<number, Texture>();
  const wanted = [...new Set(pages.filter((page) => page >= 0))];
  if (!ftxPath || wanted.length === 0) return textures;

  const loader = new THREE.TextureLoader();
  await Promise.all(
    wanted.map(async (page) => {
      let url: string | null = null;
      try {
        const buffer = await api.getFramePng(ftxPath, page);
        url = URL.createObjectURL(new Blob([buffer], { type: "image/png" }));
        const texture = await loader.loadAsync(url);
        // FMD 的 UV V 轴与图片同向（未翻转），Three 默认 flipY 会把它翻过来。
        texture.flipY = false;
        texture.colorSpace = THREE.SRGBColorSpace;
        textures.set(page, texture);
      } catch {
        // 缺页或不支持的编码：保留纯色材质。
      } finally {
        if (url) URL.revokeObjectURL(url);
      }
    }),
  );
  return textures;
}

/** 建 Mesh：有 UV 时按 sub range 的纹理页分组，缺贴图的段用纯色。 */
export function buildMesh(
  THREE: typeof import("three"),
  mesh: VizMesh,
  tint: Color,
  textures: Map<number, Texture>,
): { object: Mesh; materials: MeshStandardMaterial[] } {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(mesh.positions, 3));
  if (mesh.uvs.length * 3 === mesh.positions.length * 2) {
    geometry.setAttribute("uv", new THREE.Float32BufferAttribute(mesh.uvs, 2));
  }
  geometry.setIndex(mesh.indices);
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();

  const materials: MeshStandardMaterial[] = [];
  const byPage = new Map<number, number>();
  let offset = 0;
  for (const range of mesh.subRanges) {
    let index = byPage.get(range.page);
    if (index === undefined) {
      const texture = textures.get(range.page);
      const material = new THREE.MeshStandardMaterial(
        texture
          ? { map: texture, roughness: 0.9, metalness: 0.0, side: THREE.DoubleSide }
          : {
              color: tint,
              roughness: 0.82,
              metalness: 0.05,
              flatShading: true,
              side: THREE.DoubleSide,
            },
      );
      index = materials.length;
      materials.push(material);
      byPage.set(range.page, index);
    }
    geometry.addGroup(offset, range.count, index);
    offset += range.count;
  }

  return { object: new THREE.Mesh(geometry, materials), materials };
}
