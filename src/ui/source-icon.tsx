import type { HTMLAttributes } from "react";
import type { Source } from "../core/types";
import { typeKind } from "../core/types";

export type SourceKind =
  "table" | "data" | "enum" | "flags" | "custom" | "valueObject" | "file";
const labels: Record<SourceKind, string> = {
  table: "テーブル",
  data: "データ",
  enum: "Enum（単一選択）",
  flags: "Flags（複数選択）",
  custom: "Custom Type（構造）",
  valueObject: "Value Object（値の包み）",
  file: "ソースファイル",
};
// AntD clones icons with a slot class; preserve it for standard menu spacing.
export function SourceKindIcon({
  kind,
  className = "",
  ...props
}: { kind: SourceKind } & HTMLAttributes<HTMLSpanElement>) {
  return (
    <span
      {...props}
      className={`anticon source-icon ${className}`}
      role="img"
      aria-label={labels[kind]}
      title={labels[kind]}
      data-source-kind={kind}
    >
      <svg
        width="1em"
        height="1em"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.25"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        focusable="false"
      >
        {kind === "table" ? (
          <>
            <rect x="1.75" y="2.25" width="12.5" height="11.5" rx="1" />
            <path d="M2 6h12M6 6v7.5M10 6v7.5M2 10h12" />
          </>
        ) : kind === "data" ? (
          <>
            <rect x="1.75" y="2.25" width="12.5" height="11.5" rx="1" />
            <path d="M6.5 5h5M6.5 8h5M6.5 11h5" />
            <path d="M4 5h.1M4 8h.1M4 11h.1" strokeWidth="2" />
          </>
        ) : kind === "enum" ? (
          <>
            <circle cx="3.5" cy="3.5" r="1.5" />
            <circle cx="3.5" cy="8" r="1.5" />
            <circle cx="3.5" cy="12.5" r="1.5" />
            <circle
              cx="3.5"
              cy="3.5"
              r=".5"
              fill="currentColor"
              stroke="none"
            />
            <path d="M7 3.5h6M7 8h6M7 12.5h6" />
          </>
        ) : kind === "flags" ? (
          <>
            <rect x="1.5" y="2" width="5" height="5" rx=".6" />
            <rect x="1.5" y="9" width="5" height="5" rx=".6" />
            <path d="m2.7 4.4 1 1 1.7-2M2.7 11.4l1 1 1.7-2M9 4.5h5M9 11.5h5" />
          </>
        ) : kind === "custom" ? (
          <>
            <path d="M5 1.5H4c-1 0-1 1-1 2.5v1.5c0 1.5-.5 2-1.5 2.5C2.5 8.5 3 9 3 10.5V12c0 1.5 0 2.5 1 2.5h1M11 1.5h1c1 0 1 1 1 2.5v1.5c0 1.5.5 2 1.5 2.5-1 .5-1.5 1-1.5 2.5V12c0 1.5 0 2.5-1 2.5h-1M6 5.5h4M6 10.5h4" />
          </>
        ) : kind === "valueObject" ? (
          <>
            <rect x="1.75" y="2.25" width="12.5" height="11.5" rx="3" />
            <path d="m5.25 5.5 2.75 5 2.75-5" />
          </>
        ) : (
          <>
            <path d="M9.5 1.75H3v12.5h10v-9zM9.5 1.75v3.5H13M5.5 8h5M5.5 10.75h5" />
          </>
        )}
      </svg>
    </span>
  );
}
export function SourceIcon({
  source,
  ...props
}: { source?: Source } & HTMLAttributes<HTMLSpanElement>) {
  const kind =
    source?.data?.kind === "schema"
      ? "table"
      : source?.data?.kind === "data"
        ? "data"
        : (typeKind(source) ?? "file");
  return <SourceKindIcon kind={kind} {...props} />;
}
