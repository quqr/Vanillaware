import { useEffect, useMemo, useRef, useState } from "react";
import type { Mesh, MeshStandardMaterial, Object3D, Texture } from "three";
import { api, type FileEntry, type VizMesh, type VizReference, type VizScene } from "../../lib/ipc";
import { buildMesh, loadPageTextures } from "../../lib/meshScene";
import { useLibraryStore } from "../../store/libraryStore";

const ROLE_LABEL: Record<string, string> = {
  model: "模型",
  texture: "纹理",
  animation: "动画",
  audio: "音频",
  script: "脚本",
  text: "文本",
};

/** OSB 场景组装：节点树 + 引用 FMD 灰模；Three.js 动态加载。 */
export function OsbSceneViewer({ entry }: { entry: FileEntry }) {
  const [data, setData] = useState<VizScene | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [wireframe, setWireframe] = useState(false);
  const [stats, setStats] = useState({ meshes: 0, triangles: 0, refs: 0 });
  const [epoch, setEpoch] = useState(0);
  const select = useLibraryStore((s) => s.select);

  const mountRef = useRef<HTMLDivElement>(null);
  const materialsRef = useRef<MeshStandardMaterial[]>([]);
  const resetRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    let alive = true;
    setData(null);
    setError(null);
    setWireframe(false);
    api
      .docScene(entry.path)
      .then((next) => {
        if (!alive) return;
        const meshById = new Map(next.nodes.map((node) => [node.id, node.mesh]));
        const meshOf = (node: VizScene["nodes"][number]) =>
          node.mesh ?? (node.meshRef === null ? null : meshById.get(node.meshRef) ?? null);
        const meshes = next.nodes.filter((node) => meshOf(node) !== null).length;
        const triangles = next.nodes.reduce(
          (sum, node) => sum + (meshOf(node)?.indices.length ?? 0) / 3,
          0,
        );
        setStats({ meshes, triangles, refs: next.references.length });
        setData(next);
      })
      .catch((e) => {
        if (alive) setError(String(e));
      });
    return () => {
      alive = false;
    };
  }, [entry.path, epoch]);

  useEffect(() => {
    const mount = mountRef.current;
    if (!mount || !data) return;

    let disposed = false;
    let cleanup: (() => void) | null = null;

    void (async () => {
      const THREE = await import("three");
      const { OrbitControls } = await import("three/addons/controls/OrbitControls.js");
      if (disposed) return;

      const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
      renderer.setPixelRatio(Math.min(2, window.devicePixelRatio));
      renderer.setSize(mount.clientWidth, mount.clientHeight, false);
      renderer.domElement.className = "block h-full w-full";
      mount.appendChild(renderer.domElement);

      const scene = new THREE.Scene();
      const camera = new THREE.PerspectiveCamera(
        45,
        Math.max(1, mount.clientWidth) / Math.max(1, mount.clientHeight),
        0.01,
        1_000_000,
      );
      const controls = new OrbitControls(camera, renderer.domElement);
      controls.enableDamping = true;
      controls.dampingFactor = 0.08;
      controls.screenSpacePanning = true;

      scene.add(new THREE.AmbientLight(0xffffff, 1.5));
      const key = new THREE.DirectionalLight(0xffffff, 2.2);
      key.position.set(1, 2, 1.5);
      scene.add(key);
      const fill = new THREE.DirectionalLight(0xffffff, 0.7);
      fill.position.set(-1, -0.5, -1);
      scene.add(fill);

      const objects = new Map<number, Object3D>();
      const materials: MeshStandardMaterial[] = [];
      materialsRef.current = materials;

      // 纹理引用挂在 model 节点下：父节点即 model 节点。
      const textureOfModel = new Map<number, string>();
      for (const node of data.nodes) {
        if (node.parent !== null && node.reference?.role === "texture") {
          textureOfModel.set(node.parent, node.reference.path);
        }
      }

      const meshById = new Map<number, VizMesh>(
        data.nodes.flatMap((node) => (node.mesh ? [[node.id, node.mesh] as const] : [])),
      );
      const pagesByPath = new Map<string, Set<number>>();
      for (const node of data.nodes) {
        const mesh = node.mesh ?? (node.meshRef === null ? null : meshById.get(node.meshRef));
        if (!mesh || node.parent === null) continue;
        const ftx = textureOfModel.get(node.parent);
        if (!ftx) continue;
        const pages = pagesByPath.get(ftx) ?? new Set<number>();
        for (const range of mesh.subRanges) pages.add(range.page);
        pagesByPath.set(ftx, pages);
      }

      const loaded = await Promise.all(
        [...pagesByPath].map(
          async ([path, pages]) =>
            [path, await loadPageTextures(THREE, path, [...pages])] as const,
        ),
      );
      const texturesByPath = new Map<string, Map<number, Texture>>(loaded);
      if (disposed) {
        for (const pages of texturesByPath.values()) {
          for (const texture of pages.values()) texture.dispose();
        }
        return;
      }

      const emptyTextures = new Map<number, Texture>();
      const built = new Map<number, Mesh>();
      for (const node of data.nodes) {
        let object: Object3D;
        if (node.mesh) {
          const ftx = node.parent === null ? null : textureOfModel.get(node.parent) ?? null;
          const textures = ftx ? texturesByPath.get(ftx) ?? emptyTextures : emptyTextures;
          const tint = new THREE.Color().setHSL(((node.id * 0.137) % 1 + 1) % 1, 0.35, 0.6);
          const result = buildMesh(THREE, node.mesh, tint, textures);
          materials.push(...result.materials);
          built.set(node.id, result.object);
          object = result.object;
        } else if (node.meshRef !== null) {
          // 同一 FMD 的后续实例复用首个实例的几何与材质。
          const source = built.get(node.meshRef);
          object = source ? new THREE.Mesh(source.geometry, source.material) : new THREE.Group();
        } else {
          object = new THREE.Group();
        }

        object.matrixAutoUpdate = false;
        object.matrix.fromArray(node.transform);
        objects.set(node.id, object);
        const parent = node.parent === null ? scene : objects.get(node.parent);
        (parent ?? scene).add(object);
      }

      const [minX, minY, minZ, maxX, maxY, maxZ] = data.bounds;
      const center = new THREE.Vector3((minX + maxX) / 2, (minY + maxY) / 2, (minZ + maxZ) / 2);
      const size = Math.max(maxX - minX, maxY - minY, maxZ - minZ, 1);
      const fit = () => {
        camera.position.set(center.x + size * 0.9, center.y + size * 0.65, center.z + size * 1.4);
        camera.near = Math.max(0.01, size / 1000);
        camera.far = Math.max(1000, size * 20);
        camera.updateProjectionMatrix();
        controls.target.copy(center);
        controls.update();
      };
      fit();
      resetRef.current = fit;

      const observer = new ResizeObserver(() => {
        const width = Math.max(1, mount.clientWidth);
        const height = Math.max(1, mount.clientHeight);
        renderer.setSize(width, height, false);
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
      });
      observer.observe(mount);

      renderer.setAnimationLoop(() => {
        controls.update();
        renderer.render(scene, camera);
      });

      cleanup = () => {
        renderer.setAnimationLoop(null);
        observer.disconnect();
        controls.dispose();
        scene.traverse((object) => {
          if (object instanceof THREE.Mesh) {
            object.geometry.dispose();
            const material = object.material;
            if (Array.isArray(material)) {
              for (const item of material) item.dispose();
            } else {
              material.dispose();
            }
          }
        });
        for (const pages of texturesByPath.values()) {
          for (const texture of pages.values()) texture.dispose();
        }
        renderer.dispose();
        renderer.domElement.remove();
      };
    })().catch((e) => {
      if (!disposed) setError(String(e));
    });

    return () => {
      disposed = true;
      cleanup?.();
      materialsRef.current = [];
      resetRef.current = null;
    };
  }, [data]);

  useEffect(() => {
    for (const material of materialsRef.current) {
      material.wireframe = wireframe;
      material.needsUpdate = true;
    }
  }, [wireframe]);

  // 同一资源会被多个节点引用，按 role + path 去重后展示，点击跳到对应文件。
  const references = useMemo(() => {
    if (!data) return [];
    const grouped = new Map<string, { reference: VizReference; count: number }>();
    for (const reference of data.references) {
      const key = `${reference.role}|${reference.path}`;
      const existing = grouped.get(key);
      grouped.set(key, { reference, count: (existing?.count ?? 0) + 1 });
    }
    return [...grouped.values()];
  }, [data]);

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5 font-mono text-[10px]">
        <span className="badge badge-xs badge-primary">OSB</span>
        <span className="badge badge-xs">mesh {stats.meshes}</span>
        <span className="badge badge-xs">tri {Math.round(stats.triangles)}</span>
        <span className="badge badge-xs">ref {stats.refs}</span>
        {data?.truncated && <span className="badge badge-xs badge-warning">已截断</span>}
        <label className="ml-auto flex cursor-pointer items-center gap-1">
          <input
            type="checkbox"
            className="toggle toggle-xs"
            checked={wireframe}
            onChange={(event) => setWireframe(event.target.checked)}
          />
          线框
        </label>
        <button className="btn btn-xs" onClick={() => resetRef.current?.()}>
          重置视角
        </button>
        <button className="btn btn-xs" onClick={() => setEpoch((value) => value + 1)}>
          重载
        </button>
      </div>

      {references.length > 0 && (
        <div className="flex flex-none flex-wrap items-center gap-1 border-b border-base-300 bg-base-100 px-1.5 py-1 font-mono text-[10px]">
          <span className="text-base-content/50">引用</span>
          {references.map(({ reference, count }) => (
            <button
              key={`${reference.role}|${reference.path}`}
              className={`btn btn-xs ${reference.status === "resolved" ? "btn-outline" : "btn-disabled"}`}
              title={reference.status === "resolved" ? reference.path : `缺失：${reference.path}`}
              disabled={reference.status !== "resolved"}
              onClick={() => select(reference.path)}
            >
              <span>{ROLE_LABEL[reference.role] ?? reference.role}</span>
              <span className="max-w-40 truncate">{fileLabel(reference.path)}</span>
              {count > 1 && <span className="opacity-60">×{count}</span>}
            </button>
          ))}
        </div>
      )}

      <div className="relative min-h-0 flex-1 overflow-hidden bg-base-100" ref={mountRef}>
        {!data && !error && (
          <div className="absolute inset-0 flex items-center justify-center font-mono text-sm text-base-content/50">
            组装场景…
          </div>
        )}
        {error && (
          <div className="absolute inset-0 flex items-center justify-center p-4 text-sm text-error">
            {error}
          </div>
        )}
        {data && stats.meshes === 0 && (
          <div className="absolute inset-0 flex items-center justify-center p-4 text-sm text-base-content/50">
            只解析到资源引用，没有可渲染网格（可在「结构」中查看引用）
          </div>
        )}
      </div>
    </div>
  );
}

/** 引用面板只显示文件名，完整路径放在 title 里。 */
function fileLabel(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
