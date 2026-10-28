import { useState } from "react";

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
      <div className="font-mono text-[11px]">
        {name !== undefined && <span className="text-info">{name}: </span>}
        <span className="text-base-content/50">null</span>
      </div>
    );
  }

  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
    return (
      <div className="font-mono text-[11px]">
        {name !== undefined && <span className="text-info">{name}: </span>}
        <span className={typeof value === "string" ? "text-success" : "text-warning"}>
          {typeof value === "string" ? JSON.stringify(value) : String(value)}
        </span>
      </div>
    );
  }

  const brackets = isArray ? ["[", "]"] : ["{", "}"];

  return (
    <div className="font-mono text-[11px]">
      <span className="cursor-pointer text-base-content/70 hover:text-primary" onClick={() => setOpen((v) => !v)}>
        {open ? "▾ " : "▸ "}
        {name !== undefined && <span className="text-info">{name}: </span>}
        <span className="text-base-content/50">
          {brackets[0]}
          {open ? "" : `${count}`}
        </span>
        {!open && <span className="text-base-content/50">{brackets[1]}</span>}
      </span>

      {open && (
        <>
          <div className="ml-3.5 border-l border-base-300 pl-1.5">
            {entries
              ? entries.map(([key, child]) => (
                  <JsonTree key={key} name={key} value={child} depth={depth + 1} />
                ))
              : (value as unknown[]).map((child, index) => (
                  <JsonTree key={index} name={String(index)} value={child} depth={depth + 1} />
                ))}
          </div>
          <div className="text-base-content/50">{brackets[1]}</div>
        </>
      )}
    </div>
  );
}
