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

/** 建 Mesh：按「纹理页 + tint + 渲染路径」分组建材质，缺贴图的段退回纯色。
 *
 * 渲染路径（MTRL.tex_ref）：-1 default / 0 NML / 1 ADD / 2 SUB / 3 OPA。
 * NML 用 alphaTest 做镂空（贴图 alpha=0 的地方不画），ADD/SUB 用混合且不写深度。
 */
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
  const byKey = new Map<string, number>();
  let offset = 0;
  for (const range of mesh.subRanges) {
    const key = `${range.page}|${range.rgba.join(",")}|${range.texRef}`;
    let index = byKey.get(key);
    if (index === undefined) {
      const [red, green, blue, alpha] = range.rgba;
      const texture = textures.get(range.page);
      const blend = range.texRef;
      const mixed = blend === 1 || blend === 2;
      const material = new THREE.MeshStandardMaterial({
        // 有贴图时 rgba 作为 tint 乘上去；无贴图时用节点色区分部件。
        color: texture ? new THREE.Color(red / 255, green / 255, blue / 255) : tint,
        map: texture,
        opacity: alpha / 255,
        transparent: mixed || alpha < 255,
        depthWrite: !mixed,
        alphaTest: blend === 0 ? 0.5 : 0,
        roughness: texture ? 0.9 : 0.82,
        metalness: texture ? 0.0 : 0.05,
        flatShading: !texture,
        side: THREE.DoubleSide,
        blending:
          blend === 1
            ? THREE.AdditiveBlending
            : blend === 2
              ? THREE.SubtractiveBlending
              : THREE.NormalBlending,
      });
      index = materials.length;
      materials.push(material);
      byKey.set(key, index);
    }
    geometry.addGroup(offset, range.count, index);
    offset += range.count;
  }

  return { object: new THREE.Mesh(geometry, materials), materials };
}
