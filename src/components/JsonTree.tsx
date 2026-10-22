import { useState } from "react";

/** 只读 JSON 树：默认展开两层，超过 50 项的数组默认折叠，避免几 MB 的 Quad 一次性铺开。 */
export function JsonTree({
  name,
  value,
  depth = 0,
}: {
  name?: string;
  value: unknown;
  depth?: number;
}) {
  const isArray = Array.isArray(value);
  const entries =
    value !== null && typeof value === "object" && !isArray ? Object.entries(value as Record<string, unknown>) : null;

  const count = isArray ? (value as unknown[]).length : (entries?.length ?? 0);
  const [open, setOpen] = useState(depth < 2 && count <= 50);

  if (value === null) {
    return (
      <div className="node">
        {name !== undefined && <span className="node-key">{name}: </span>}
        <span className="node-null">null</span>
      </div>
    );
  }

  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
    return (
      <div className="node">
        {name !== undefined && <span className="node-key">{name}: </span>}
        <span className={typeof value === "string" ? "node-str" : "node-num"}>
          {typeof value === "string" ? JSON.stringify(value) : String(value)}
        </span>
      </div>
    );
  }

  const brackets = isArray ? ["[", "]"] : ["{", "}"];

  return (
    <div className="node">
      <span className="node-toggle" onClick={() => setOpen((v) => !v)}>
        {open ? "▾ " : "▸ "}
        {name !== undefined && <span className="node-key">{name}: </span>}
        <span className="dim">
          {brackets[0]}
          {open ? "" : `${count}`}
        </span>
        {!open && <span className="dim">{brackets[1]}</span>}
      </span>

      {open && (
        <>
          <div className="node-children">
            {entries
              ? entries.map(([key, child]) => (
                  <JsonTree key={key} name={key} value={child} depth={depth + 1} />
                ))
              : (value as unknown[]).map((child, index) => (
                  <JsonTree key={index} name={String(index)} value={child} depth={depth + 1} />
                ))}
          </div>
          <div className="dim">{brackets[1]}</div>
        </>
      )}
    </div>
  );
}
