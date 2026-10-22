# ksc 生成器：把 spec/ksy/*.ksy 全部生成到 src-tauri/crates/vw-gen/src/gen/，并重写 gen/mod.rs。
# 用法：python tools/ksc_gen.py（仓库根目录下运行）
# 生成物禁止手改，要改解析行为请改 .ksy。
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
KSC = ROOT / "kaitai-struct-compiler-0.11" / "bin" / "kaitai-struct-compiler.bat"
KSY_DIR = ROOT / "spec" / "ksy"
OUT_DIR = ROOT / "src-tauri" / "crates" / "vw-gen" / "src" / "gen"
HEADER = "// 由 ksc 生成，禁止手改。新增 .ksy 后运行 `python tools/ksc_gen.py` 重新生成本文件。\n"


def main() -> int:
    if not KSC.exists():
        print(f"ksc 不存在：{KSC}", file=sys.stderr)
        return 1
    ksys = sorted(KSY_DIR.glob("*.ksy"))
    if not ksys:
        print(f"没有 .ksy：{KSY_DIR}", file=sys.stderr)
        return 1
    for ksy in ksys:
        cmd = [str(KSC), "-t", "rust", "--outdir", str(OUT_DIR), str(ksy)]
        print(" ".join(cmd))
        rc = subprocess.call(cmd)
        if rc != 0:
            return rc
    mods = "".join(f"pub mod {k.stem};\n" for k in ksys)
    (OUT_DIR / "mod.rs").write_text(HEADER + mods, encoding="utf-8", newline="\n")
    print(f"已写入 {OUT_DIR / 'mod.rs'}（{len(ksys)} 个模块）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
