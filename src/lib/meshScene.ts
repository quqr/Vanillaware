import type { Color, Mesh, MeshStandardMaterial, Texture } from "three";
import { api, type VizMesh } from "./ipc";

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
        texture.flipY = false;
        texture.colorSpace = THREE.SRGBColorSpace;
        textures.set(page, texture);
      } catch {
      } finally {
        if (url) URL.revokeObjectURL(url);
      }
    }),
  );
  return textures;
}

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
