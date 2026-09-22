import { Alert } from "antd";

export type CompatibilityDiagnostic = {
  code: string;
  message: string;
  source?: string;
  schemaPath?: string;
  schema_path?: string;
};

type ClassificationCount = { classification: string; count: number };
type CompatibilitySummary = {
  changeCount: number;
  generatedApi: ClassificationCount[];
  sourceMigration: ClassificationCount[];
  artifactBinary: ClassificationCount[];
  externalContract: ClassificationCount[];
};
export type MigrationCompatibility = {
  report?: { summary: CompatibilitySummary } | null;
  diagnostic?: CompatibilityDiagnostic | null;
};

const axis = (label: string, counts: ClassificationCount[]) =>
  `${label}: ${counts.length === 0 ? "unchanged" : counts.map(item => `${item.classification} ${item.count}`).join(", ")}`;

export default function MigrationCompatibilityImpact({ value }: { value?: MigrationCompatibility | null }) {
  if (!value) return null;
  if (!value.report) {
    return value.diagnostic
      ? <Alert
          type="info"
          title="Released compatibility impact unavailable"
          description={`${value.diagnostic.code}: ${value.diagnostic.message}`}
        />
      : null;
  }
  const summary = value.report.summary;
  return <section aria-label="Released Compatibility Impact" className="migration-compatibility-impact">
    <h4>Released compatibility impact</h4>
    <p>{summary.changeCount} reported compatibility changes</p>
    <p>{axis("Generated API", summary.generatedApi)}</p>
    <p>{axis("Source / Migration", summary.sourceMigration)}</p>
    <p>{axis("Artifact / Binary", summary.artifactBinary)}</p>
    <p>{axis("External Contract", summary.externalContract)}</p>
  </section>;
}
