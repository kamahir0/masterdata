import React from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { DeliveryPanel, ProjectCreatePanel, ProjectSettingsPanel, type SurfaceWorkspace } from "../src/ProjectSurfaces";

const { invoke, openDialog } = vi.hoisted(() => ({ invoke: vi.fn(), openDialog: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: openDialog }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const configSnapshot = {
  projectRoot: "/project",
  configPath: "/project/masterdata.toml",
  baseSource: "base0",
  baseContentIdentity: "id0",
  configValid: true,
  profiles: [{ name: "prod", include_tags: ["old"], exclude_tags: [] }],
  publishTargets: [{
    occurrence: 0,
    kind: "csharp",
    path: "dist",
    resolved_path: "/project/dist",
    editable: true,
    reason: null,
    location: "publish.targets[0]",
  }],
  diagnostics: [],
};

const workspace: SurfaceWorkspace = {
  project: {
    project_root: "/project",
    config_path: "/project/masterdata.toml",
    name: "Demo",
    project_id: "demo",
    profiles: [{ name: "prod", include_tags: ["release"], exclude_tags: [] }],
  },
  files: [],
};

beforeEach(() => {
  invoke.mockReset();
  openDialog.mockReset();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test("Settings composes profile and target edits in one file buffer before Save", async () => {
  const onDirtyChange = vi.fn();
  const onRegisterSave = vi.fn();
  const onSaved = vi.fn(async () => true);
  let previewCount = 0;
  invoke.mockImplementation(async (command, args) => {
    if (command === "open_project_config") return structuredClone(configSnapshot);
    if (command === "preview_project_config_edit") {
      previewCount += 1;
      if (previewCount === 1) {
        expect(args.baseSource).toBe("base0");
        expect(args.baseContentIdentity).toBe("id0");
        expect(args.request.operation).toBe("update_profile");
        return {
          baseContentIdentity: "id0",
          candidateContentIdentity: "id1",
          candidateSource: "base1",
          changed: true,
          configValid: true,
          diagnostics: [],
        };
      }
      expect(args.baseSource).toBe("base1");
      expect(args.baseContentIdentity).toBe("id1");
      expect(args.request).toEqual({ operation: "update_publish_target_path", index: 0, path: "dist2" });
      return {
        baseContentIdentity: "id1",
        candidateContentIdentity: "id2",
        candidateSource: "base2",
        changed: true,
        configValid: true,
        diagnostics: [],
      };
    }
    if (command === "save_project_config_edit") {
      expect(args.baseSource).toBe("base0");
      expect(args.baseContentIdentity).toBe("id0");
      expect(args.requests).toEqual([
        { operation: "update_profile", name: "prod", include_tags: ["new"], exclude_tags: [] },
        { operation: "update_publish_target_path", index: 0, path: "dist2" },
      ]);
      return {
        status: "success",
        snapshot: {
          ...structuredClone(configSnapshot),
          baseSource: "base2",
          baseContentIdentity: "id2",
          profiles: [{ name: "prod", include_tags: ["new"], exclude_tags: [] }],
          publishTargets: [{ ...configSnapshot.publishTargets[0], path: "dist2" }],
        },
        current: null,
        diagnostic: null,
      };
    }
    throw new Error(`Unexpected command: ${command}`);
  });

  render(
    <ProjectSettingsPanel
      active
      projectRoot="/project"
      mutationBlocked={false}
      onDirtyChange={onDirtyChange}
      onRegisterSave={onRegisterSave}
      onSaved={onSaved}
    />,
  );

  await screen.findByDisplayValue("old");
  fireEvent.change(screen.getByLabelText("Include tags"), { target: { value: "new" } });
  fireEvent.click(screen.getByRole("button", { name: "Apply Profile to buffer" }));
  await screen.findByText("base1");

  fireEvent.click(screen.getByRole("button", { name: "Edit path" }));
  fireEvent.change(screen.getByLabelText("Publish target path"), { target: { value: "dist2" } });
  fireEvent.click(screen.getByRole("button", { name: "Apply Target Path to buffer" }));
  await screen.findByText("base2");

  fireEvent.click(screen.getByRole("button", { name: "Save Settings" }));
  await waitFor(() => expect(onSaved).toHaveBeenCalledOnce());
  expect(invoke.mock.calls.filter(([command]) => command === "save_project_config_edit")).toHaveLength(1);
});

test("Settings profile navigation restores the buffered draft instead of the saved snapshot", async () => {
  const snapshot = {
    ...structuredClone(configSnapshot),
    profiles: [
      { name: "prod", include_tags: ["old"], exclude_tags: [] },
      { name: "staging", include_tags: ["stage"], exclude_tags: [] },
    ],
  };
  invoke.mockImplementation(async (command, args) => {
    if (command === "open_project_config") return snapshot;
    if (command === "preview_project_config_edit") {
      expect(args.request.operation).toBe("update_profile");
      return {
        baseContentIdentity: args.baseContentIdentity,
        candidateContentIdentity: "id1",
        candidateSource: "base1",
        changed: true,
        configValid: true,
        diagnostics: [],
      };
    }
    throw new Error(`Unexpected command: ${command}`);
  });

  render(
    <ProjectSettingsPanel
      active
      projectRoot="/project"
      mutationBlocked={false}
      onDirtyChange={() => {}}
      onRegisterSave={() => {}}
      onSaved={async () => true}
    />,
  );
  await screen.findByDisplayValue("old");
  fireEvent.change(screen.getByLabelText("Include tags"), { target: { value: "new" } });

  fireEvent.mouseDown(screen.getByRole("combobox", { name: "Settings profile" }));
  fireEvent.click((await screen.findAllByText("staging", { selector: ".ant-select-item-option-content" })).at(-1)!);
  await waitFor(() => expect((screen.getByLabelText("Include tags") as HTMLInputElement).value).toBe("stage"));

  fireEvent.mouseDown(screen.getByRole("combobox", { name: "Settings profile" }));
  fireEvent.click((await screen.findAllByText("prod", { selector: ".ant-select-item-option-content" })).at(-1)!);
  await waitFor(() => expect((screen.getByLabelText("Include tags") as HTMLInputElement).value).toBe("new"));
});

test("Settings Reload requires explicit discard while a form draft is dirty", async () => {
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
  invoke.mockImplementation(async (command) => {
    if (command === "open_project_config") return structuredClone(configSnapshot);
    throw new Error(`Unexpected command: ${command}`);
  });

  render(
    <ProjectSettingsPanel
      active
      projectRoot="/project"
      mutationBlocked={false}
      onDirtyChange={() => {}}
      onRegisterSave={() => {}}
      onSaved={async () => true}
    />,
  );
  await screen.findByDisplayValue("old");
  fireEvent.change(screen.getByLabelText("Include tags"), { target: { value: "unsaved" } });
  fireEvent.click(screen.getByRole("button", { name: "Reload Settings" }));

  expect(confirm).toHaveBeenCalledOnce();
  expect(invoke.mock.calls.filter(([command]) => command === "open_project_config")).toHaveLength(1);
  expect((screen.getByLabelText("Include tags") as HTMLInputElement).value).toBe("unsaved");
});

test("Delivery clears an old Build success when the next operation fails", async () => {
  let buildCount = 0;
  invoke.mockImplementation(async (command) => {
    if (command === "build") {
      buildCount += 1;
      if (buildCount === 1) {
        return { profile: null, generatedFiles: ["a"], artifactRoot: "/project/.masterdata/output" };
      }
      throw { diagnostic: { code: "E-BUILD", message: "failed build" } };
    }
    throw new Error(`Unexpected command: ${command}`);
  });

  render(
    <DeliveryPanel
      active
      projectRoot="/project"
      workspace={workspace}
      dirtySourceCount={0}
      dirtyConfig={false}
      profile=""
      onProfileChange={() => {}}
      buildBlocked={false}
      publishBlocked={false}
      onBusyChange={() => {}}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "Build saved input" }));
  await screen.findByText(/Build succeeded/);
  fireEvent.click(screen.getByRole("button", { name: "Build saved input" }));
  await screen.findByText("Build failed");
  expect(screen.queryByText(/Build succeeded/)).toBeNull();
});

test("Delivery retains target-level Publish failure results and expires the old confirmation", async () => {
  invoke.mockImplementation(async (command, args) => {
    if (command === "publish_preview") {
      return {
        artifactSetIdentity: "artifact",
        configContentIdentity: "config",
        publishPlanIdentity: "plan",
        targets: [
          { index: 0, kind: "csharp", configuredPath: "first", destination: "/first", additions: ["A"], updates: [], removals: [], binaryReplacement: false, preflightOk: true },
          { index: 1, kind: "binary", configuredPath: "second", destination: "/second", additions: [], updates: [], removals: [], binaryReplacement: true, preflightOk: true },
        ],
      };
    }
    if (command === "publish_from_preview") {
      expect(args.publishPlanIdentity).toBe("plan");
      return {
        status: "failure",
        outcome: "partial_failure",
        unityVerification: "not_observed",
        report: {
          targets: [
            { index: 0, kind: "csharp", configured_path: "first", destination: "/first", status: "succeeded", failure: null },
            { index: 1, kind: "binary", configured_path: "second", destination: "/second", status: "failed", failure: { code: "E-PUBLISH", message: "target failed" } },
          ],
        },
        diagnostic: { code: "E-PUBLISH-EXECUTION-FAILED", message: "one target failed" },
      };
    }
    throw new Error(`Unexpected command: ${command}`);
  });

  render(
    <DeliveryPanel
      active
      projectRoot="/project"
      workspace={workspace}
      dirtySourceCount={0}
      dirtyConfig={false}
      profile=""
      onProfileChange={() => {}}
      buildBlocked={false}
      publishBlocked={false}
      onBusyChange={() => {}}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "Publish preview" }));
  const confirm = await screen.findByRole("button", { name: "Confirm Publish" });
  fireEvent.click(confirm);
  await screen.findByText("Publish result");
  expect(screen.getByText("succeeded")).toBeTruthy();
  expect(screen.getByText("failed")).toBeTruthy();
  expect(screen.getByText("partial_failure")).toBeTruthy();
  expect(screen.getByText(/Unity verification: not observed/)).toBeTruthy();
  expect(screen.getByText(/E-PUBLISH: target failed/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Confirm Publish" })).toBeNull();
});

test("Publish transport failure is shown as unknown and cannot reuse the old confirmation", async () => {
  invoke.mockImplementation(async (command) => {
    if (command === "publish_preview") {
      return {
        artifactSetIdentity: "artifact",
        configContentIdentity: "config",
        publishPlanIdentity: "plan",
        targets: [{ index: 0, kind: "binary", configuredPath: "dist", destination: "/dist", additions: [], updates: [], removals: [], binaryReplacement: true, preflightOk: true }],
      };
    }
    if (command === "publish_from_preview") {
      throw { diagnostic: { code: "E-TRANSPORT", message: "connection lost" } };
    }
    throw new Error(`Unexpected command: ${command}`);
  });

  render(
    <DeliveryPanel
      active
      projectRoot="/project"
      workspace={workspace}
      dirtySourceCount={0}
      dirtyConfig={false}
      profile=""
      onProfileChange={() => {}}
      buildBlocked={false}
      publishBlocked={false}
      onBusyChange={() => {}}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "Publish preview" }));
  fireEvent.click(await screen.findByRole("button", { name: "Confirm Publish" }));
  expect(await screen.findByText("Publish outcome unknown")).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Confirm Publish" })).toBeNull();
});

test("Recovery-style Build blocking still leaves Publish-only preview eligible", async () => {
  invoke.mockImplementation(async (command) => {
    if (command === "publish_preview") {
      return { artifactSetIdentity: "a", configContentIdentity: "c", publishPlanIdentity: "p", targets: [] };
    }
    throw new Error(`Unexpected command: ${command}`);
  });

  render(
    <DeliveryPanel
      active
      projectRoot="/project"
      workspace={workspace}
      dirtySourceCount={0}
      dirtyConfig={false}
      profile=""
      onProfileChange={() => {}}
      buildBlocked
      publishBlocked={false}
      onBusyChange={() => {}}
    />,
  );

  expect((screen.getByRole("button", { name: "Build saved input" }) as HTMLButtonElement).disabled).toBe(true);
  const publish = screen.getByRole("button", { name: "Publish preview" }) as HTMLButtonElement;
  expect(publish.disabled).toBe(false);
  fireEvent.click(publish);
  expect(await screen.findByText("0 targets: successful no-op.")).toBeTruthy();
});

test("Delivery shares busy state while a Build is unresolved", async () => {
  let resolveBuild!: (value: unknown) => void;
  const onBusyChange = vi.fn();
  invoke.mockImplementation(async (command) => {
    if (command === "build") return new Promise((resolve) => { resolveBuild = resolve; });
    throw new Error(`Unexpected command: ${command}`);
  });

  render(
    <DeliveryPanel
      active
      projectRoot="/project"
      workspace={workspace}
      dirtySourceCount={0}
      dirtyConfig={false}
      profile=""
      onProfileChange={() => {}}
      buildBlocked={false}
      publishBlocked={false}
      onBusyChange={onBusyChange}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "Build saved input" }));
  await waitFor(() => expect(onBusyChange).toHaveBeenCalledWith(true));
  resolveBuild({ profile: null, generatedFiles: [], artifactRoot: "/project/.masterdata/output" });
  await waitFor(() => expect(onBusyChange).toHaveBeenLastCalledWith(false));
});


test.each(['cancel', 'select', 'error'])('Create Project folder picker handles %s without creating a Project', async (outcome) => {
  if (outcome === 'error') openDialog.mockRejectedValue(new Error('Picker unavailable'));
  else openDialog.mockResolvedValue(outcome === 'select' ? '/chosen' : null);
  render(<ProjectCreatePanel active onCreated={vi.fn()} onCancel={vi.fn()} />);
  const destination = screen.getByRole('textbox', { name: 'Project destination' });
  fireEvent.change(destination, { target: { value: '/typed/new' } });
  fireEvent.click(screen.getByRole('button', { name: 'Choose Folder…' }));
  await waitFor(() => expect(openDialog).toHaveBeenCalledOnce());
  if (outcome === 'error') expect(await screen.findByText('Error: Picker unavailable')).toBeTruthy();
  await waitFor(() => {
    expect((destination as HTMLInputElement).disabled).toBe(false);
    expect((destination as HTMLInputElement).value).toBe(outcome === 'select' ? '/chosen' : '/typed/new');
  });
  expect(invoke).not.toHaveBeenCalled();
});

test("Create Project disables cancellation and edits until shared creation finishes", async () => {
  const onCancel = vi.fn();
  const onCreated = vi.fn();
  let finish!: (result: unknown) => void;
  invoke.mockImplementation(() => new Promise(resolve => { finish = resolve; }));
  render(<ProjectCreatePanel active onCreated={onCreated} onCancel={onCancel} />);
  fireEvent.change(screen.getByLabelText('Project destination'), { target: { value: '/new' } });
  fireEvent.change(screen.getByLabelText('New project ID'), { target: { value: 'new' } });
  fireEvent.change(screen.getByLabelText('New project name'), { target: { value: 'New' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create Project' }));
  expect(invoke).toHaveBeenCalledWith('create_project', { request: { destination: '/new', projectId: 'new', name: 'New', version: '0.1.0' } });
  expect((screen.getByRole('button', { name: 'Cancel' }) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByLabelText('New project name') as HTMLInputElement).disabled).toBe(true);
  finish({ status: 'success', destination: '/new', project: { project_root: '/new' }, createdEntries: [] });
  await waitFor(() => expect(onCreated).toHaveBeenCalledWith('/new'));
  expect(onCancel).not.toHaveBeenCalled();
});
