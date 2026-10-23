import { useEffect, useRef, useState } from "react";
import type { MeshStandardMaterial } from "three";
import { api, type FileEntry, type VizScene } from "../../lib/ipc";

/** FMD 首版：纯色三角面 + 线框切换 + 轨道相机（ADR-0019）。
 *
 * Three.js 只在打开模型时动态加载，避免把 3D 引擎塞进首屏 bundle。
 */
export function FmdMeshViewer({ entry }: { entry: FileEntry }) {
  const [data, setData] = useState<VizScene | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [wireframe, setWireframe] = useState(false);
  const [stats, setStats] = useState({ meshes: 0, triangles: 0 });
  const [epoch, setEpoch] = useState(0);

  const mountRef = useRef<HTMLDivElement>(null);
  const materialsRef = useRef<MeshStandardMaterial[]>([]);
  const resetRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    let alive = true;
    setData(null);
    setError(null);
    setWireframe(false);
    api
      .docMesh(entry.path)
      .then((next) => {
        if (!alive) return;
        const meshes = next.nodes.filter((node) => node.mesh).length;
        const triangles = next.nodes.reduce(
          (sum, node) => sum + (node.mesh ? node.mesh.indices.length / 3 : 0),
          0,
        );
        setStats({ meshes, triangles });
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

      scene.add(new THREE.AmbientLight(0xffffff, 1.4));
      const key = new THREE.DirectionalLight(0xffffff, 2.2);
      key.position.set(1, 2, 1.5);
      scene.add(key);
      const fill = new THREE.DirectionalLight(0xffffff, 0.8);
      fill.position.set(-1, -0.5, -1);
      scene.add(fill);

      const materials: MeshStandardMaterial[] = [];
      materialsRef.current = materials;

      for (const node of data.nodes) {
        if (!node.mesh || node.mesh.indices.length === 0) continue;
        const geometry = new THREE.BufferGeometry();
        geometry.setAttribute(
          "position",
          new THREE.Float32BufferAttribute(node.mesh.positions, 3),
        );
        geometry.setIndex(node.mesh.indices);
        geometry.computeVertexNormals();
        geometry.computeBoundingSphere();

        const color = new THREE.Color().setHSL(((node.id * 0.137) % 1 + 1) % 1, 0.42, 0.62);
        const material = new THREE.MeshStandardMaterial({
          color,
          roughness: 0.82,
          metalness: 0.05,
          flatShading: true,
          wireframe: false,
          side: THREE.DoubleSide,
        });
        materials.push(material);
        scene.add(new THREE.Mesh(geometry, material));
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
        for (const child of scene.children) {
          if (child instanceof THREE.Mesh) {
            child.geometry.dispose();
            const material = child.material;
            if (Array.isArray(material)) {
              for (const item of material) item.dispose();
            } else {
              material.dispose();
            }
          }
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

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex flex-none flex-wrap items-center gap-1.5 border-b border-base-300 bg-base-200 p-1.5 font-mono text-[10px]">
        <span className="badge badge-xs badge-primary">FMD</span>
        <span className="badge badge-xs">mesh {stats.meshes}</span>
        <span className="badge badge-xs">tri {Math.round(stats.triangles)}</span>
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

      <div className="relative min-h-0 flex-1 overflow-hidden bg-base-100" ref={mountRef}>
        {!data && !error && (
          <div className="absolute inset-0 flex items-center justify-center font-mono text-sm text-base-content/50">
            读取网格…
          </div>
        )}
        {error && (
          <div className="absolute inset-0 flex items-center justify-center p-4 text-sm text-error">
            {error}
          </div>
        )}
        {data && stats.meshes === 0 && (
          <div className="absolute inset-0 flex items-center justify-center p-4 text-sm text-base-content/50">
            该文件没有非空网格
          </div>
        )}
      </div>
    </div>
  );
}
