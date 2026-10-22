import React, {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import {
  App as AntApp,
  Alert,
  Badge,
  Button,
  Card,
  Collapse,
  ConfigProvider,
  Drawer,
  Dropdown,
  Empty,
  Input,
  Modal,
  Popconfirm,
  Progress,
  Result,
  Space,
  Spin,
  Tabs,
  Tag,
  Tooltip,
  Typography,
  theme as antTheme,
} from "antd";
import jaJP from "antd/locale/ja_JP";
import { Provider as ComponentMotionProvider } from "@rc-component/motion";
import {
  ArrowRightOutlined,
  CheckCircleOutlined,
  CloseOutlined,
  CodeOutlined,
  DatabaseOutlined,
  ExclamationCircleOutlined,
  ExperimentOutlined,
  FolderOpenOutlined,
  FileDoneOutlined,
  DownOutlined,
  PlayCircleOutlined,
  PlusOutlined,
  RedoOutlined,
  RocketOutlined,
  SaveOutlined,
  SettingOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  ControlOutlined,
  UndoOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { AnimatePresence, motion, MotionConfig } from "motion/react";
import { session } from "./core/session";
import { identity } from "./core/identity";
import {
  type BuildResult,
  type LoadedProject,
  type Problem,
  type Source,
  tables,
  typeKind,
  text,
} from "./core/types";
import { serialize } from "./core/yaml";
import { Explorer, SourceIcon } from "./ui/explorer";
import { GridEditor } from "./ui/grid";
import { TypeEditor } from "./ui/type-editor";
import { Relationships } from "./ui/relationships";
import { FieldEditor } from "./ui/fields";
import { NestedEditor, type Cursor } from "./ui/nested";
import {
  EditorSettings,
  useEditorPreferences,
  type EditorPreferences,
} from "./ui/editor-settings";
import { workbenchThemes, workbenchMetrics } from "./ui/design-tokens";
import { ExplorerResize } from "./ui/explorer-resize";
import { sourceKind, kindBadges } from "./ui/source-icon";
import {
  isBrowser,
  browserSupport,
  reauthorizeAccess,
} from "./browser/runtime";
import { BrowserRecovery } from "./browser/recovery";
import appIcon from "../assets/icon.svg";
import { SearchSelect } from "./ui/search-select";
import type { FileOperations } from "./ui/file-context-menu";
import { FileTabs } from "./ui/file-tabs";
import { CloseFileDialog } from "./ui/close-file";
import { Settings } from "./ui/settings";
import { Entry } from "./ui/inputs";
import { EditorBoundary } from "./ui/editor-boundary";
import { quickTransition, easeOutExpoCss, resultTransition } from "./ui/motion";
function SourceText({ source }: { source: Source }) {
  const readText = () => {
    const saved = session.getState().saved.get(source.id);
    return saved && !session.dirtySource(source)
      ? saved.text
      : serialize(source, saved);
  };
  const [draft, setDraft] = useState(readText);
  const [changed, setChanged] = useState(false);
  useEffect(() => {
    if (!changed) setDraft(readText());
  }, [source]);
  const apply = () => {
    if (changed) {
      session.raw(source.id, draft);
      setChanged(false);
    }
  };
  return (
    <div className="source-editor">
      <div className="source-editor-toolbar">
        <Typography.Text type="secondary">
          YAMLソース · 変更は通常のUndoで戻せます
        </Typography.Text>
        <Button
          type={changed ? "primary" : "default"}
          onClick={apply}
          disabled={!changed}
        >
          編集を反映
        </Button>
      </div>
      <Input.TextArea
        aria-label="YAMLソース"
        value={draft}
        spellCheck={false}
        onChange={(e) => {
          setDraft(e.target.value);
          setChanged(true);
          window.desktop?.dirty(true);
        }}
        onBlur={apply}
        onKeyDown={(e) => {
          if ((e.metaKey || e.ctrlKey) && e.key === "Enter") apply();
        }}
      />
    </div>
  );
}
function Workspace({
  preferences,
  setPreferences,
}: {
  preferences: EditorPreferences;
  setPreferences: React.Dispatch<React.SetStateAction<EditorPreferences>>;
}) {
  const browser = isBrowser();
  const reduced = preferences.reduced;
  const setReduced = (reduced: boolean) =>
    setPreferences((old) => ({ ...old, reduced }));
  const state = useSyncExternalStore(session.subscribe, session.getState);
  const { snapshot, project, revision } = state;
  const { message, modal } = AntApp.useApp();
  const [active, setActive] = useState("");
  const [opened, setOpened] = useState<string[]>([]);
  const [preview, setPreview] = useState<string | null>(null);
  const [closing, setClosing] = useState<{ id: string; path: string } | null>(
    null,
  );
  const [editorSettings, setEditorSettings] = useState(false);
  const navigation = useRef({ active, opened, preview });
  navigation.current = { active, opened, preview };
  const [tab, setTab] = useState("data");
  const explorerOperations = useRef<FileOperations>(null);
  const [cursor, setCursor] = useState<Cursor | null>(null);
  const [settings, setSettings] = useState(false);
  const [problemsOpen, setProblemsOpen] = useState(false);
  const [problems, setProblems] = useState<Problem[]>([]);
  const [validating, setValidating] = useState(false);
  const [saving, setSaving] = useState(false);
  const savingRequest = useRef<Promise<void> | null>(null);
  const [loading, setLoading] = useState(false);
  const [newOpen, setNewOpen] = useState(false);
  const [newName, setNewName] = useState("新しいプロジェクト");
  const [profile, setProfile] = useState("");
  useEffect(() => {
    if (profile && !Object.hasOwn(snapshot.config.profiles, profile))
      setProfile("");
  }, [profile, snapshot.config.profiles]);
  const [buildOpen, setBuildOpen] = useState(false);
  const [building, setBuilding] = useState(false);
  const [publishing, setPublishing] = useState(false);
  // Native commands may arrive together before React commits the busy state.
  const outputBusy = useRef(false);
  const [buildResult, setBuildResult] = useState<BuildResult | null>(null);
  const [buildError, setBuildError] = useState("");
  const [publishResult, setPublishResult] = useState<{
    message: string;
    revision: number;
    profile: string;
  } | null>(null);
  const [jump, setJump] = useState<{
    source: string;
    path: (string | number)[];
    nonce: number;
  }>();
  const find = useRef<any>(null);
  const guard = useRef(false);
  const newestRevision = useRef(revision);
  newestRevision.current = revision;
  const source = snapshot.sources.find((s) => s.id === active);
  const dirty = session.dirty;
  useEffect(() => {
    if (
      !preferences.explorer &&
      document.activeElement?.closest(".explorer-container")
    )
      (
        document.querySelector(
          '.file-tab[aria-selected="true"]',
        ) as HTMLElement | null
      )?.focus();
  }, [preferences.explorer]);
  const fail = (error: unknown) => {
    message.error({
      content: error instanceof Error ? error.message : String(error),
      duration: 7,
    });
  };
  const commitActive = () => {
    (document.activeElement as HTMLElement)?.blur?.();
  };
  useEffect(() => {
    const callback = (event: Event) => fail((event as CustomEvent).detail);
    window.addEventListener("masterdata:error", callback);
    return () => window.removeEventListener("masterdata:error", callback);
  }, []);
  useEffect(() => {
    window.desktop?.dirty(dirty);
  }, [dirty, revision]);
  useEffect(() => {
    const worker = new Worker(
      new URL("./core/validation-worker.ts", import.meta.url),
      { type: "module" },
    );
    worker.onmessage = ({ data }) => {
      if (data.revision === newestRevision.current) {
        setProblems(data.problems);
        setValidating(false);
      }
    };
    worker.onerror = (event) => {
      setValidating(false);
      fail(event.message);
    };
    const unsubscribe = session.subscribe(() => {
      const state = session.getState();
      setValidating(true);
      clearTimeout(timer);
      timer = setTimeout(
        () =>
          worker.postMessage({
            snapshot: state.snapshot,
            revision: state.revision,
          }),
        180,
      );
    });
    let timer: ReturnType<typeof setTimeout>;
    if (session.getState().project)
      worker.postMessage({
        snapshot: session.getState().snapshot,
        revision: session.getState().revision,
      });
    return () => {
      clearTimeout(timer);
      unsubscribe();
      worker.terminate();
    };
  }, []);
  // Commit local input before deciding whether a preview is replaceable.
  const select = (id: string, pin = false) => {
    // Tree / tab activation already blurs the value editor. Keep their new
    // focus so Escape and arrow navigation stay owned by the navigation pane.
    if (!document.activeElement?.closest(".explorer-container, .file-tabs"))
      commitActive();
    const current = session.getState().snapshot;
    const target = current.sources.find((source) => source.id === id);
    if (!target) return;
    const previous = current.sources.find((source) => source.id === preview);
    const keepPrevious = previous && session.dirtySource(previous);
    const permanent = pin || session.dirtySource(target);
    if (!opened.includes(id)) {
      setOpened((old) => {
        const next =
          preview && !keepPrevious ? old.filter((key) => key !== preview) : old;
        const at = preview && !keepPrevious ? old.indexOf(preview) : -1;
        const result = [...next];
        if (at >= 0) result.splice(at, 0, id);
        else result.push(id);
        return result;
      });
      setPreview(permanent ? null : id);
    } else if ((preview === id && permanent) || keepPrevious) setPreview(null);
    setJump(undefined);
    if (id !== active) {
      setActive(id);
      setCursor(null);
      setTab("data");
    }
  };
  useEffect(() => {
    const current = snapshot.sources.find((source) => source.id === preview);
    if (preview && (!current || session.dirtySource(current))) setPreview(null);
  }, [snapshot, preview]);
  const load = (value: LoadedProject | null) => {
    if (!value) return;
    session.load(value);
    setOpened([]);
    setPreview(null);
    setCursor(null);
    setProfile("");
    setBuildResult(null);
    setPublishResult(null);
    setBuildError("");
    setJump(undefined);
    const first =
      value.snapshot.sources.find((s) => s.data?.kind === "schema") ??
      value.snapshot.sources[0];
    if (first) {
      setActive(first.id);
      setOpened([first.id]);
      setPreview(first.id);
    } else setActive("");
  };
  useEffect(() => {
    window.desktop?.initial().then(load).catch(fail);
  }, []);
  async function save(scope: boolean | string = false, report = true) {
    // A queued Save All (including close confirmation) must observe the first
    // acknowledgement and then capture newer drafts, never silently skip them.
    while (savingRequest.current) await savingRequest.current;
    commitActive();
    const current = session.getState();
    if (!current.project) return;
    const currentSource = current.snapshot.sources.find(
      (source) => source.id === active,
    );
    const ids =
      typeof scope === "string"
        ? [scope]
        : scope
          ? undefined
          : currentSource
            ? [currentSource.id]
            : [];
    if (ids && !ids.length) return;
    const request = (async () => {
      setSaving(true);
      try {
        const captured = current.snapshot;
        const result = await window.desktop.save({
          token: current.project!.token,
          files: session.files(ids),
          folders: session.saveFolders(captured, ids),
          config: session.saveConfig(captured, ids),
        });
        session.acceptSave(result, captured, ids);
        message.success("保存しました");
      } catch (error) {
        if (report) fail(error);
        throw error;
      } finally {
        setSaving(false);
      }
    })();
    savingRequest.current = request;
    try {
      await request;
    } finally {
      if (savingRequest.current === request) savingRequest.current = null;
    }
  }
  function removeTab(id: string) {
    // A queued close can finish after the user has opened another file.
    const current = navigation.current;
    const remaining = current.opened.filter(
      (key) =>
        key !== id &&
        session.getState().snapshot.sources.some((source) => source.id === key),
    );
    setOpened(remaining);
    if (current.preview === id) setPreview(null);
    if (current.active === id) {
      requestAnimationFrame(() => {
        const next =
          document.querySelector<HTMLElement>(
            '.file-tab[aria-selected="true"]',
          ) ?? document.querySelector<HTMLElement>(".main-area");
        next?.focus();
      });

      const index = current.opened.indexOf(id);
      setActive(remaining[Math.min(index, remaining.length - 1)] ?? "");
      setCursor(null);
      setTab("data");
      setJump(undefined);
    }
  }
  async function closeTab(id: string) {
    if (!id || guard.current) return;
    const projectToken = session.getState().project?.token;
    try {
      while (savingRequest.current) await savingRequest.current;
    } catch {
      return;
    }
    if (guard.current || projectToken !== session.getState().project?.token)
      return;
    commitActive();
    const current = session
      .getState()
      .snapshot.sources.find((source) => source.id === id);
    if (current && session.dirtySource(current)) {
      guard.current = true;
      setClosing({ id, path: current.path });
    } else removeTab(id);
  }
  const finishClose = () => {
    setClosing(null);
    guard.current = false;
  };
  async function guarded(action: () => Promise<void> | void) {
    if (guard.current) return;
    commitActive();
    if (!session.dirty) {
      try {
        while (savingRequest.current) await savingRequest.current;
      } catch {
        return;
      }
      commitActive();
      if (session.dirty) return guarded(action);
      await action();
      return;
    }
    guard.current = true;
    const instance = modal.confirm({
      title: "変更を保存しますか？",
      content:
        "未保存の変更があります。保存してから続けるか、今回の変更を破棄できます。",
      okText: "保存して続ける",
      cancelText: "キャンセル",
      onOk: async () => {
        try {
          await save(true);
          commitActive();
          if (session.dirty)
            throw new Error(
              "保存中に変更が追加されました。入力を確認して再度保存してください。",
            );
          guard.current = false;
          await action();
        } catch {
          guard.current = false;
          throw new Error("保存できませんでした。入力は保持しています。");
        }
      },
      onCancel: () => {
        guard.current = false;
      },
      footer: (_, { OkBtn, CancelBtn }) => (
        <Space>
          <Button
            danger
            onClick={async () => {
              guard.current = false;
              instance.destroy();
              await action();
            }}
          >
            破棄して続ける
          </Button>
          <CancelBtn />
          <OkBtn />
        </Space>
      ),
    });
  }
  async function open() {
    await guarded(async () => {
      setLoading(true);
      try {
        load(await window.desktop.open());
      } catch (error) {
        fail(error);
      } finally {
        setLoading(false);
      }
    });
  }
  async function sample() {
    await guarded(async () => {
      setLoading(true);
      try {
        load(await window.desktop.sample());
      } catch (error) {
        fail(error);
      } finally {
        setLoading(false);
      }
    });
  }
  async function build() {
    if (browser || outputBusy.current) return;
    commitActive();
    const current = session.getState();
    if (!current.project) return;
    outputBusy.current = true;
    setBuilding(true);
    setBuildOpen(true);
    setBuildError("");
    try {
      setBuildResult(
        await window.desktop.build(
          current.project.token,
          current.snapshot,
          Object.hasOwn(current.snapshot.config.profiles, profile)
            ? profile
            : "",
          current.revision,
        ),
      );
      message.success("Buildが完了しました");
    } catch (error) {
      setBuildError(error instanceof Error ? error.message : String(error));
    } finally {
      outputBusy.current = false;
      setBuilding(false);
    }
  }
  async function publish() {
    if (browser || !project || !buildResult || outputBusy.current) return;
    outputBusy.current = true;
    const publishedBuild = buildResult;
    setPublishing(true);
    setBuildOpen(true);
    setPublishResult(null);
    setBuildError("");
    try {
      const result = await window.desktop.publish(
        project.token,
        session.getState().snapshot.config.publish,
      );
      setPublishResult({
        message: result,
        revision: publishedBuild.revision,
        profile: publishedBuild.profile,
      });
      message.success({ content: result, duration: 7 });
    } catch (error) {
      setBuildError(error instanceof Error ? error.message : String(error));
    } finally {
      outputBusy.current = false;
      setPublishing(false);
    }
  }
  const actions = useRef({ open, save, build, guarded, closeTab, active });
  actions.current = { open, save, build, guarded, closeTab, active };
  useEffect(
    () =>
      window.desktop?.onCommand((command) => {
        if (command === "open") void actions.current.open();
        if (!browser && command === "new")
          void actions.current.guarded(() => setNewOpen(true));
        if (command === "save") void actions.current.save().catch(() => {});
        if (command === "save-all")
          void actions.current.save(true).catch(() => {});
        if (command === "close-file")
          void actions.current.closeTab(actions.current.active);
        if (command === "explorer")
          setPreferences((old) => ({ ...old, explorer: !old.explorer }));
        if (command === "editor-settings") setEditorSettings(true);
        if (!browser && command === "build-results") setBuildOpen(true);
        if (!browser && command === "build") void actions.current.build();
        if (command === "undo") {
          commitActive();
          session.undo();
        }
        if (command === "redo") {
          commitActive();
          session.redo();
        }
        if (command === "find") find.current?.focus();
        if (command === "problems") setProblemsOpen((old) => !old);
        if (command === "close")
          void actions.current.guarded(() => window.desktop.close());
        if (command === "busy-close")
          message.info("保存・生成処理が完了するまでお待ちください。");
      }),
    [],
  );
  const jumpTo = (problem: Problem) => {
    select(problem.source);
    setJump({ source: problem.source, path: problem.path, nonce: Date.now() });
  };
  const title =
    source?.data?.kind === "type" ? source.data.name : source?.data?.table;
  const tabs =
    source?.data?.kind === "type"
      ? [
          { key: "data", label: "型の定義" },
          { key: "source", label: "ソース", icon: <CodeOutlined /> },
        ]
      : [
          { key: "data", label: "データ" },
          { key: "fields", label: "フィールド" },
          { key: "relations", label: "キーと参照" },
          { key: "source", label: "ソース", icon: <CodeOutlined /> },
        ];
  return (
    <div className="application">
      <header className="titlebar">
        <div className="brand">
          <span className="brand-mark">
            <img src={appIcon} alt="" />
          </span>
          <strong>MasterData</strong>
          {project && (
            <>
              <span className="brand-divider" />{" "}
              <span className="window-project">{snapshot.config.name}</span>
            </>
          )}
        </div>
        <Space className="titlebar-controls">
          <Tooltip title="エディター設定">
            <Button
              type="text"
              aria-label="エディター設定"
              icon={<SettingOutlined />}
              onClick={() => setEditorSettings(true)}
            />
          </Tooltip>
          {project && (
            <Tooltip title="プロジェクト設定">
              <Button
                type="text"
                aria-label="プロジェクト設定"
                icon={<ControlOutlined />}
                onClick={() => setSettings(true)}
              />
            </Tooltip>
          )}
        </Space>
      </header>
      {project ? (
        <>
          <div className="app-toolbar">
            <Space size={4}>
              <Tooltip
                title={
                  preferences.explorer
                    ? "エクスプローラーを折りたたむ"
                    : "エクスプローラーを表示"
                }
              >
                <Button
                  type="text"
                  aria-label="エクスプローラーの表示切替"
                  aria-expanded={preferences.explorer}
                  icon={
                    preferences.explorer ? (
                      <MenuFoldOutlined />
                    ) : (
                      <MenuUnfoldOutlined />
                    )
                  }
                  onClick={() =>
                    setPreferences((old) => ({
                      ...old,
                      explorer: !old.explorer,
                    }))
                  }
                />
              </Tooltip>
              <Button icon={<FolderOpenOutlined />} onClick={open}>
                開く
              </Button>
              <Space.Compact>
                <Button
                  aria-label="保存"
                  title="現在のファイルを保存"
                  type="primary"
                  icon={<SaveOutlined />}
                  loading={saving}
                  disabled={saving || !source}
                  onClick={() => void save().catch(() => {})}
                >
                  保存
                </Button>
                <Dropdown
                  trigger={["click"]}
                  menu={{
                    items: [
                      { key: "all", label: "すべて保存", disabled: saving },
                      ...(browser
                        ? [
                            {
                              key: "access",
                              label: "フォルダのアクセスを再許可",
                              disabled: saving,
                            },
                          ]
                        : []),
                    ],
                    onClick: ({ key }) => {
                      if (key === "access")
                        void reauthorizeAccess()
                          .then((granted) => {
                            if (granted)
                              message.success(
                                "フォルダへのアクセスを許可しました。入力を確認して保存してください。",
                              );
                          })
                          .catch(fail);
                      else void save(true).catch(() => {});
                    },
                  }}
                >
                  <Button
                    type="primary"
                    aria-label="保存方法"
                    icon={<DownOutlined />}
                  />
                </Dropdown>
              </Space.Compact>
              <span className="toolbar-divider" />
              <Tooltip
                title={
                  state.past.length
                    ? `元に戻す: ${state.past.at(-1)?.label}`
                    : "元に戻す"
                }
              >
                <Button
                  type="text"
                  icon={<UndoOutlined />}
                  aria-label="元に戻す"
                  disabled={!state.past.length}
                  onClick={() => {
                    commitActive();
                    session.undo();
                  }}
                />
              </Tooltip>
              <Tooltip title="やり直す">
                <Button
                  type="text"
                  icon={<RedoOutlined />}
                  aria-label="やり直す"
                  disabled={!state.future.length}
                  onClick={() => session.redo()}
                />
              </Tooltip>
              <Tooltip title={dirty ? "未保存の変更" : "保存済み"}>
                <span
                  className="save-state"
                  role="status"
                  aria-label={dirty ? "未保存の変更" : "保存済み"}
                >
                  {dirty ? (
                    <span className="dirty-dot" />
                  ) : (
                    <CheckCircleOutlined />
                  )}
                </span>
              </Tooltip>
            </Space>
            {!browser && (
              <Space>
                <Tooltip title="未選択時はタグで絞り込まずにビルドします">
                  <SearchSelect
                    aria-label="出力プロファイル"
                    className="profile-select"
                    placeholder="プロファイルを選択"
                    value={profile || undefined}
                    allowClear
                    options={Object.keys(snapshot.config.profiles).map(
                      (value) => ({ value, label: value }),
                    )}
                    onChange={(value) => setProfile(value ?? "")}
                  />
                </Tooltip>
                <Space.Compact className="build-controls">
                  <Button
                    icon={<PlayCircleOutlined />}
                    loading={building}
                    disabled={publishing}
                    onClick={build}
                  >
                    Build
                  </Button>
                  <Dropdown
                    trigger={["click"]}
                    menu={{
                      items: [
                        {
                          key: "publish",
                          icon: <RocketOutlined aria-hidden="true" />,
                          label: "Publish…",
                        },
                        { type: "divider" },
                        {
                          key: "results",
                          icon: <FileDoneOutlined aria-hidden="true" />,
                          label: "実行結果を表示",
                        },
                      ],
                      onClick: () => setBuildOpen(true),
                    }}
                  >
                    <Button
                      icon={
                        <Badge dot={!!buildError} color="red">
                          <DownOutlined />
                        </Badge>
                      }
                      aria-label="ビルドの操作"
                    />
                  </Dropdown>
                </Space.Compact>
              </Space>
            )}
          </div>
          <div className="workspace">
            <div
              className={`explorer-container ${preferences.explorer ? "" : "explorer-collapsed"}`}
              style={{
                width: preferences.explorer
                  ? `min(${preferences.explorerWidth}px, max(180px, calc(100vw - 520px)))`
                  : 0,
              }}
              inert={!preferences.explorer}
              aria-hidden={!preferences.explorer}
            >
              <Explorer
                ref={explorerOperations}
                key={project.token}
                snapshot={snapshot}
                saved={state.saved}
                active={active}
                onSelect={select}
                fail={fail}
              />
            </div>
            {preferences.explorer && (
              <ExplorerResize
                value={preferences.explorerWidth}
                update={(width) =>
                  setPreferences((p) => ({ ...p, explorerWidth: width }))
                }
              />
            )}
            <main className="main-area" tabIndex={-1}>
              <div className="editor-surface">
                <FileTabs
                  sources={opened
                    .map((id) => snapshot.sources.find((s) => s.id === id))
                    .filter((s): s is Source => !!s)}
                  active={active}
                  preview={preview}
                  wrap={preferences.tabLayout === "wrap"}
                  reduced={preferences.reduced}
                  dirty={(source) => session.dirtySource(source)}
                  select={select}
                  pin={(id) => select(id, true)}
                  close={closeTab}
                  fail={fail}
                  operations={{
                    rename: (path) => {
                      setPreferences((p) => ({ ...p, explorer: true }));
                      explorerOperations.current?.rename(path);
                    },
                    remove: (paths) => explorerOperations.current?.remove(paths),
                  }}
                />
                {source ? (
                  <>
                    <div className="editor-heading">
                      <div
                        className="editor-title"
                        data-source-kind={sourceKind(source)}
                      >
                        <Tag className="source-kind-label">
                          {kindBadges[sourceKind(source)]}
                        </Tag>
                        <span className="source-badge">
                          <SourceIcon source={source} />
                        </span>
                        <Entry
                          value={title ?? source.path.split("/").at(-1)}
                          label="定義名"
                          onCommit={(name) =>
                            session.renameDeclaration(source.id, name)
                          }
                        />
                      </div>
                      <Tabs
                        className="editor-tabs"
                        activeKey={tab}
                        onChange={(value) => {
                          commitActive();
                          setTab(value);
                          setCursor(null);
                        }}
                        items={tabs}
                      />
                    </div>
                    <section className="editor-content">
                      <EditorBoundary
                        key={`${active}:${tab}`}
                        revision={revision}
                        onSource={() => setTab("source")}
                      >
                        {tab === "source" || source.syntaxError ? (
                          <>
                            <SourceText key={source.id} source={source} />
                          </>
                        ) : source.data?.kind === "type" ? (
                          <TypeEditor source={source} snapshot={snapshot} />
                        ) : tab === "data" ? (
                          <GridEditor
                            key={source.id}
                            source={source}
                            snapshot={snapshot}
                            problems={problems}
                            onOpen={setCursor}
                            navigate={select}
                            profile={profile}
                            findRef={find}
                            jump={jump}
                          />
                        ) : tab === "fields" ? (
                          <div className="editor-scroll">
                            <FieldEditor
                              source={
                                tables(snapshot).find(
                                  (t) => t.name === source.data?.table,
                                )?.source ?? source
                              }
                              snapshot={snapshot}
                            />
                          </div>
                        ) : (
                          <Relationships
                            source={
                              tables(snapshot).find(
                                (t) => t.name === source.data?.table,
                              )?.source ?? source
                            }
                            snapshot={snapshot}
                          />
                        )}
                      </EditorBoundary>
                      <NestedEditor
                        reduced={preferences.reduced}
                        cursor={cursor}
                        snapshot={snapshot}
                        setCursor={setCursor}
                      />
                    </section>
                  </>
                ) : (
                  <div className="empty-editor">
                    <Empty
                      image={Empty.PRESENTED_IMAGE_SIMPLE}
                      description="左の一覧から表や型を選択してください"
                    />
                    <Button
                      type="primary"
                      icon={<PlusOutlined />}
                      onClick={() => select(session.add("table"))}
                    >
                      テーブルを作成
                    </Button>
                  </div>
                )}
              </div>
              <AnimatePresence initial={false}>
                {problemsOpen && (
                  <motion.section
                    className="problems-panel"
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: 230, opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={quickTransition}
                  >
                    <div className="problems-heading">
                      <Space>
                        <WarningOutlined />
                        <strong>問題</strong>
                        <Badge
                          count={problems.length}
                          showZero
                          color={problems.length ? "#e76f51" : "#52a88a"}
                        />
                        {validating && <Spin size="small" />}
                      </Space>
                      <Button
                        size="small"
                        type="text"
                        icon={<CloseOutlined />}
                        aria-label="問題を閉じる"
                        onClick={() => setProblemsOpen(false)}
                      />
                    </div>
                    <div className="problems-list">
                      {problems.length ? (
                        problems.map((problem, i) => (
                          <button
                            className="problem-item"
                            key={i}
                            onClick={() => jumpTo(problem)}
                          >
                            <ExclamationCircleOutlined />
                            <span>{problem.message}</span>
                            <small>
                              {
                                snapshot.sources.find(
                                  (s) => s.id === problem.source,
                                )?.path
                              }{" "}
                              {problem.path.map(String).join(" › ")}
                            </small>
                          </button>
                        ))
                      ) : (
                        <div className="problems-clear">
                          <CheckCircleOutlined /> 問題は見つかりませんでした
                        </div>
                      )}
                    </div>
                  </motion.section>
                )}
              </AnimatePresence>
            </main>
          </div>
          <footer className="statusbar">
            <button onClick={() => setProblemsOpen(!problemsOpen)}>
              <WarningOutlined /> {problems.length} 件の問題
              {validating ? " · 確認中" : ""}
            </button>
            <span>{source?.path ?? snapshot.config.name}</span>
            <Space>
              <span>UTF-8</span>
              <span>YAML</span>
              <button onClick={() => setReduced(!reduced)}>
                {`モーション: ${reduced ? "OFF" : "ON"}`}
              </button>
            </Space>
          </footer>
        </>
      ) : (
        <div className="welcome">
          <motion.div
            initial={{ opacity: 0, y: 18 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.35 }}
            className="welcome-main"
          >
            <Typography.Title>MasterData</Typography.Title>
            <Typography.Paragraph>
              テーブルデータと型定義を統合管理するマスターデータエディタです。
              <br />
              プロジェクトを開くか、新規作成して編集を開始します。
            </Typography.Paragraph>
            <Space size="middle">
              <Button
                type="primary"
                size="large"
                icon={<FolderOpenOutlined />}
                onClick={open}
                disabled={browser && !browserSupport()}
              >
                プロジェクトを開く
              </Button>
              {!browser && (
                <Button
                  size="large"
                  icon={<PlusOutlined />}
                  onClick={() => setNewOpen(true)}
                >
                  新しく作る
                </Button>
              )}
            </Space>
            {!browser && (
              <button className="sample-link" onClick={sample}>
                <ExperimentOutlined /> サンプルで試す <ArrowRightOutlined />
              </button>
            )}
            {browser && (
              <Typography.Paragraph type="secondary" style={{ marginTop: 20 }}>
                {browserSupport()
                  ? "ローカルのプロジェクトフォルダを選択します。変更は保存時にだけ反映されます。未保存の入力はタブ終了で失われます。"
                  : "この環境では編集できません。デスクトップ版ChromeとHTTPS（またはlocalhost）で開いてください。"}
              </Typography.Paragraph>
            )}
          </motion.div>
          <motion.div
            className="welcome-card"
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: 0.12, duration: 0.35 }}
          >
            <div className="welcome-card-header">
              <span className="preview-dot" />
              <span className="preview-dot" />
              <span className="preview-dot" />
              <strong>テーブル編集プレビュー</strong>
            </div>
            <div className="preview-grid">
              <div>id</div>
              <div>name</div>
              <div>rarity</div>
              <span>1001</span>
              <strong>銅の剣</strong>
              <Tag color="blue">Common</Tag>
              <span>2001</span>
              <strong>氷晶の剣</strong>
              <Tag color="purple">Rare</Tag>
              <span>3001</span>
              <strong>夜明けの剣</strong>
              <Tag color="gold">Epic</Tag>
            </div>
            <div className="preview-footer">
              <CheckCircleOutlined /> 外部キー参照・型定義・値の整合性を管理
            </div>
          </motion.div>
        </div>
      )}
      {loading && (
        <div className="loading-cover">
          <Spin size="large" />
          <span>プロジェクトを読み込んでいます</span>
        </div>
      )}
      <Modal
        title="新しいプロジェクト"
        open={newOpen}
        okText="保存先を選択"
        cancelText="キャンセル"
        onCancel={() => setNewOpen(false)}
        onOk={async () => {
          try {
            const value = await window.desktop.create(newName);
            if (value) {
              load(value);
              setNewOpen(false);
            }
          } catch (error) {
            fail(error);
          }
        }}
      >
        <label>プロジェクト名</label>
        <Input
          aria-label="新しいプロジェクト名"
          autoFocus
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
        />
        <Typography.Paragraph
          type="secondary"
          style={{ marginTop: 12, marginBottom: 0 }}
        >
          次に選ぶ保存先の中へ、この名前のフォルダを作成します。
        </Typography.Paragraph>
      </Modal>
      <Settings
        open={settings}
        close={() => setSettings(false)}
        snapshot={snapshot}
      />
      <EditorSettings
        open={editorSettings}
        close={() => setEditorSettings(false)}
        value={preferences}
        update={setPreferences}
      />
      {closing && (
        <CloseFileDialog
          key={closing.id}
          name={closing.path}
          cancel={finishClose}
          fail={fail}
          save={async () => {
            await save(closing.id, false);
            const current = session
              .getState()
              .snapshot.sources.find((source) => source.id === closing.id);
            if (current && session.dirtySource(current))
              throw new Error(
                "保存中に変更が追加されました。変更を確認して再度保存してください。",
              );
            removeTab(closing.id);
            finishClose();
          }}
          discard={() => {
            session.discardSource(closing.id);
            removeTab(closing.id);
            finishClose();
          }}
        />
      )}
      {!browser && (
        <Drawer
          title={
            <Space>
              <RocketOutlined /> Build / Publish
            </Space>
          }
          width={570}
          open={buildOpen}
          onClose={() => setBuildOpen(false)}
        >
          <div className="build-panel">
            <Typography.Paragraph type="secondary">
              現在の編集内容をBuildし、成果物を確認してからPublishで配置します。保存は別の操作です。
            </Typography.Paragraph>
            <Space>
              <Tag color="blue">{profile || "プロファイル未選択"}</Tag>
              {!profile && (
                <Typography.Text type="secondary">
                  タグの絞り込みなし
                </Typography.Text>
              )}
              <Tag>編集 rev.{revision}</Tag>
              {dirty && <Tag color="gold">未保存の変更を含む</Tag>}
            </Space>
            <Button
              type="primary"
              size="large"
              icon={<PlayCircleOutlined />}
              loading={building}
              disabled={publishing}
              onClick={build}
              block
            >
              Buildする
            </Button>
            {building && (
              <div className="build-progress">
                <Spin />
                <Typography.Text>
                  データを検証し、.NETで成果物を生成・確認しています。
                </Typography.Text>
              </div>
            )}
            {publishResult && (
              <motion.div
                key={identity(publishResult)}
                initial={reduced ? false : { opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={reduced ? { duration: 0 } : resultTransition}
              >
                <Alert
                  type="success"
                  title={`Publish完了 · rev.${publishResult.revision} · ${publishResult.profile || "プロファイル未選択"}`}
                  description={publishResult.message}
                  showIcon
                />
              </motion.div>
            )}
            {buildError && (
              <motion.div
                initial={reduced ? false : { opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={reduced ? { duration: 0 } : resultTransition}
              >
                <Alert
                  type="error"
                  showIcon
                  title="処理を完了できませんでした"
                  description={<pre className="build-error">{buildError}</pre>}
                />
              </motion.div>
            )}
            {buildResult && (
              <>
                <motion.div
                  key={identity(buildResult)}
                  initial={reduced ? false : { opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={reduced ? { duration: 0 } : resultTransition}
                >
                  <Result
                    status="success"
                    title="成果物を生成しました"
                    subTitle={`${buildResult.tables} テーブル · ${buildResult.records} レコード · ${buildResult.profile || "プロファイル未選択"} · rev.${buildResult.revision}`}
                  />
                </motion.div>
                {(buildResult.revision !== revision ||
                  buildResult.profile !== profile) && (
                  <Alert
                    type="info"
                    title="Build後に編集内容または出力条件が変わっています"
                    description={`以下のPublishは、成功したrev.${buildResult.revision}の成果物を配置します。`}
                  />
                )}
                <Card size="small" title="配置先">
                  <div className="publish-target">
                    <strong>C#</strong>
                    <span>{snapshot.config.publish.csharp || "未設定"}</span>
                  </div>
                  <div className="publish-target">
                    <strong>Binary</strong>
                    <span>{snapshot.config.publish.binary || "未設定"}</span>
                  </div>
                  <Button type="link" onClick={() => setSettings(true)}>
                    出力先を設定
                  </Button>
                </Card>
                <Popconfirm
                  title="成果物を配置しますか？"
                  description="設定した出力先の、MasterDataが管理する成果物を更新します。"
                  okText="配置する"
                  cancelText="キャンセル"
                  onConfirm={publish}
                >
                  <Button
                    block
                    size="large"
                    icon={<RocketOutlined />}
                    loading={publishing}
                    disabled={
                      building ||
                      (!snapshot.config.publish.csharp &&
                        !snapshot.config.publish.binary)
                    }
                  >
                    Publishする
                  </Button>
                </Popconfirm>
                <Button
                  icon={<FolderOpenOutlined />}
                  onClick={() =>
                    project && window.desktop.reveal(project.token, true)
                  }
                >
                  成果物フォルダを開く
                </Button>
                <Collapse
                  items={[
                    {
                      key: "details",
                      label: "生成ファイルと実行結果",
                      children: (
                        <>
                          <pre>{buildResult.files.join("\n")}</pre>
                          <pre className="build-log">{buildResult.log}</pre>
                        </>
                      ),
                    },
                  ]}
                />
              </>
            )}
          </div>
        </Drawer>
      )}
      {browser && <BrowserRecovery fail={fail} />}
    </div>
  );
}
export default function Root() {
  const { preferences, setPreferences, dark } = useEditorPreferences();
  const reduced = preferences.reduced;
  const palette = workbenchThemes[dark ? "dark" : "light"];
  useLayoutEffect(() => {
    for (const [name, value] of Object.entries(palette))
      document.documentElement.style.setProperty(`--${name}`, value);
    for (const [name, value] of Object.entries(workbenchMetrics))
      document.documentElement.style.setProperty(`--${name}`, `${value}px`);
    document.documentElement.dataset.runtime = isBrowser()
      ? "browser"
      : "desktop";
    document.documentElement.dataset.platform = navigator.platform
      .toLowerCase()
      .includes("mac")
      ? "mac"
      : "other";
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    document.documentElement.dataset.reducedMotion = String(reduced);
  }, [dark, reduced, palette]);
  return (
    <ConfigProvider
      locale={jaJP}
      theme={{
        algorithm: dark ? antTheme.darkAlgorithm : antTheme.defaultAlgorithm,
        token: {
          colorPrimary: palette.primary,
          colorBgContainer: palette.surface,
          colorBgElevated: palette.surface,
          colorBgLayout: palette.bg,
          colorBorder: palette.border,
          colorText: palette.text,
          colorTextSecondary: palette.muted,
          colorSplit: palette.line,
          colorPrimaryText: palette.accent,
          borderRadius: workbenchMetrics["radius-control"],
          fontFamily:
            '-apple-system, BlinkMacSystemFont, "Hiragino Sans", "Noto Sans JP", sans-serif',
          fontSize: 13,
          // Keep AntD's wrapper topology stable. A permanent component-motion
          // provider below disables lifecycle motion without remounting drafts.
          // CSS duration overrides alone race popup placement in WebKit.
          motion: true,
          motionDurationFast: ".08s",
          motionDurationMid: ".12s",
          motionDurationSlow: ".16s",
          motionEaseOut: easeOutExpoCss,
          motionEaseInOut: easeOutExpoCss,
        },
        components: {
          Table: { cellPaddingBlockSM: 5, cellPaddingInlineSM: 8 },
          Tabs: {
            itemSelectedColor: palette.accent,
            itemHoverColor: palette.accent,
            inkBarColor: palette.accent,
          },
          Radio: {
            colorPrimary: palette.accent,
            colorPrimaryHover: palette.accent,
          },
          Button: {
            defaultShadow: "none",
            primaryShadow: "none",
          },
          Tree: {
            titleHeight: 28,
            indentSize: 16,
            nodeSelectedBg: palette.selected,
            motionDurationSlow: ".1s",
          },
        },
      }}
    >
      <ComponentMotionProvider motion={!reduced}>
        <MotionConfig reducedMotion={reduced ? "always" : "user"}>
          <AntApp>
            <Workspace
              preferences={preferences}
              setPreferences={setPreferences}
            />
          </AntApp>
        </MotionConfig>
      </ComponentMotionProvider>
    </ConfigProvider>
  );
}
