// Reuse existing authoring fixture definitions at the new aggregate boundary.
// These legacy fixture keys never become native requests; navigation protocol
// and coalescing are tested directly in workspace-navigation.test.ts.
export function installWorkspaceFixture(invoke: any, fixture: (command: string, args?: any) => any) {
  let workspace: any;
  let validation: any = { valid: true, diagnostics: [] };
  invoke.mockImplementation(async (command: string, args: any) => {
    if (command === "open_workspace" || command === "refresh_workspace") {
      workspace = await fixture("authoring_workspace", args);
      return workspace;
    }
    if (command === "workspace_status") return { generation: 1, workspace, validation };
    if (command === "workspace_validation") return { generation: 1, validation, tagCandidatesComplete: true, tables: {} };
    if (command !== "select_source") return fixture(command, args);
    const file = workspace?.files?.find((entry: any) => entry.path === args.relativePath);
    let context: any = null;
    let data: any = null;
    let typeSnapshot: any = null;
    if (file?.kind === "type") typeSnapshot = await fixture("open_type", args);
    else {
      try { context = await fixture("open_table_context", args); }
      catch (error) { if (error && typeof error === "object" && "diagnostic" in error) throw error; }
      if (context) {
        // Table details now consume this snapshot instead of issuing their own read.
        try {
          const details = await fixture("open_table", { ...args, relativePath: context.schemaPath });
          context = { ...context, schema: { ...details, ...context.schema, initializerShapes: context.schema.initializerShapes ?? details?.initializerShapes ?? {} } };
        } catch { /* This fixture does not exercise advanced Table details. */ }
      }
      if (file?.kind !== "schema" || file.hasInlineRecords || context?.selectedRecordSource) {
        const path = file?.kind === "schema" && !file.hasInlineRecords ? context.selectedRecordSource : args.relativePath;
        data = await fixture("open_data_file", { ...args, relativePath: path });
        if (data) { data = { ...data, path }; validation = data.validation; }
        if (context) context = { ...context, selectedRecordSource: path };
      }
    }
    return { requestedPath: args.relativePath, path: data?.path ?? args.relativePath, generation: 1, files: workspace?.files ?? [], currentSource: null, context, data, typeSnapshot, validationPending: false };
  });
}
