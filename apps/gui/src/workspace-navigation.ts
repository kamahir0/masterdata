import { invoke } from "./navigation-trace";

export type WorkspaceSelection<Data, Context, Type, File = unknown, Source = unknown> = {
  requestedPath: string;
  path: string;
  generation: number;
  files: File[];
  currentSource: Source | null;
  context: Context | null;
  data: Data | null;
  typeSnapshot: Type | null;
  validationPending: boolean;
};

export async function readDataFile<T>(projectPath: string, relativePath: string): Promise<T> {
  const view = await invoke<WorkspaceSelection<T, unknown, unknown>>("select_source", { projectPath, relativePath });
  if (!view?.data) throw new Error("The selected record source is unavailable");
  return view.data;
}
export async function readTableContext<T>(projectPath: string, relativePath: string): Promise<T> {
  const view = await invoke<WorkspaceSelection<unknown, T, unknown>>("select_source", { projectPath, relativePath });
  if (!view?.context) throw new Error("The selected Table context is unavailable");
  return view.context;
}

type Request<T> = { args: { projectPath: string; relativePath: string; requestId: number }; resolve: (value: T | null) => void; reject: (reason: unknown) => void };

// WHY: a rapid A→B→C sequence needs one active read and only the newest pending
// read. Result rejection alone would still queue every expensive projection.
// EVIDENCE: GUI-EXPLORER-NAV-001; tests/workspace-navigation.test.ts
export class WorkspaceNavigator<T> {
  private sequence = 0;
  private pending: Request<T> | null = null;
  private running = false;
  constructor(private readonly read: (args: Request<T>["args"]) => Promise<T | null> = args => invoke<T | null>("select_source", args)) {}
  select(projectPath: string, relativePath: string): Promise<T | null> {
    this.pending?.resolve(null);
    const requestId = ++this.sequence;
    const result = new Promise<T | null>((resolve, reject) => { this.pending = { args: { projectPath, relativePath, requestId }, resolve, reject }; });
    queueMicrotask(() => void this.drain());
    return result;
  }
  cancel() { ++this.sequence; this.pending?.resolve(null); this.pending = null; }
  private async drain() {
    if (this.running) return;
    this.running = true;
    try {
      while (this.pending) {
        const request = this.pending;
        this.pending = null;
        try {
          const value = await this.read(request.args);
          request.resolve(request.args.requestId === this.sequence ? value : null);
        } catch (error) {
          if (request.args.requestId === this.sequence) request.reject(error);
          else request.resolve(null);
        }
      }
    } finally { this.running = false; }
  }
}
