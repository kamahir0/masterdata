import React, {
  useEffect,
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
  Select,
  Space,
  Spin,
  Tabs,
  Tag,
  Tooltip,
  Typography,
  theme as antTheme,
} from "antd";
import jaJP from "antd/locale/ja_JP";
import {
  ApartmentOutlined,
  ArrowRightOutlined,
  BulbOutlined,
  CheckCircleOutlined,
  CloseOutlined,
  CodeOutlined,
  DatabaseOutlined,
  ExclamationCircleOutlined,
  ExperimentOutlined,
  FolderOpenOutlined,
  DownOutlined,
  MoonOutlined,
  PlayCircleOutlined,
  PlusOutlined,
  RedoOutlined,
  RocketOutlined,
  SaveOutlined,
  SettingOutlined,
  UndoOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { AnimatePresence, motion, MotionConfig } from "motion/react";
import { session } from "./core/session";
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
import { Settings } from "./ui/settings";
import { Entry } from "./ui/inputs";
import { EditorBoundary } from "./ui/editor-boundary";
import { quickTransition, easeOutExpoCss } from "./ui/motion";
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
  dark,
  setDark,
  reduced,
  setReduced,
}: {
  dark: boolean;
  setDark: (value: boolean) => void;
  reduced: boolean;
  setReduced: (value: boolean) => void;
}) {
  const state = useSyncExternalStore(session.subscribe, session.getState);
  const { snapshot, project, revision } = state;
  const { message, modal } = AntApp.useApp();
  const [active, setActive] = useState("");
  const [opened, setOpened] = useState<string[]>([]);
  const [tab, setTab] = useState("data");
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
  const [buildOpen, setBuildOpen] = useState(false);
  const [building, setBuilding] = useState(false);
  const [publishing, setPublishing] = useState(false);
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
  const select = (id: string) => {
    setJump(undefined);
    setActive(id);
    setOpened((old) => (old.includes(id) ? old : [...old, id]));
    setCursor(null);
    setTab("data");
  };
  const load = (value: LoadedProject | null) => {
    if (!value) return;
    session.load(value);
    setOpened([]);
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
    } else setActive("");
  };
  useEffect(() => {
    window.desktop?.initial().then(load).catch(fail);
  }, []);
  async function save(all = false) {
    // A queued Save All (including close confirmation) must observe the first
    // acknowledgement and then capture newer drafts, never silently skip them.
    while (savingRequest.current) await savingRequest.current;
    commitActive();
    const current = session.getState();
    if (!current.project) return;
    const ids = all ? undefined : active ? [active] : [];
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
        fail(error);
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
  async function guarded(action: () => Promise<void> | void) {
    commitActive();
    if (!session.dirty) {
      await action();
      return;
    }
    if (guard.current) return;
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
    commitActive();
    const current = session.getState();
    if (!current.project) return;
    setBuilding(true);
    setBuildOpen(true);
    setBuildError("");
    try {
      setBuildResult(
        await window.desktop.build(
          current.project.token,
          current.snapshot,
          profile,
          current.revision,
        ),
      );
      message.success("Buildが完了しました");
    } catch (error) {
      setBuildError(error instanceof Error ? error.message : String(error));
    } finally {
      setBuilding(false);
    }
  }
  async function publish() {
    if (!project || !buildResult) return;
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
      setPublishing(false);
    }
  }
  const actions = useRef({ open, save, build, guarded });
  actions.current = { open, save, build, guarded };
  useEffect(
    () =>
      window.desktop?.onCommand((command) => {
        if (command === "open") void actions.current.open();
        if (command === "new")
          void actions.current.guarded(() => setNewOpen(true));
        if (command === "save") void actions.current.save().catch(() => {});
        if (command === "save-all")
          void actions.current.save(true).catch(() => {});
        if (command === "build-results") setBuildOpen(true);
        if (command === "build") void actions.current.build();
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
            <DatabaseOutlined />
          </span>
          <strong>MasterData</strong>
          <span className="brand-divider" />{" "}
          <span className="window-project">
            {project ? snapshot.config.name : "データから、世界をつくる。"}
          </span>
        </div>
        <Space className="titlebar-controls">
          <Tooltip title={dark ? "ライトテーマ" : "ダークテーマ"}>
            <Button
              type="text"
              icon={dark ? <BulbOutlined /> : <MoonOutlined />}
              aria-label="テーマを切り替え"
              onClick={() => setDark(!dark)}
            />
          </Tooltip>
          {project && (
            <Tooltip title="プロジェクト設定">
              <Button
                type="text"
                aria-label="プロジェクト設定"
                icon={<SettingOutlined />}
                onClick={() => setSettings(true)}
              />
            </Tooltip>
          )}
        </Space>
      </header>
      {project ? (
        <>
          <div className="app-toolbar">
            <Space>
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
                    ],
                    onClick: () => void save(true).catch(() => {}),
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
            <Space>
              <Select
                aria-label="出力プロファイル"
                className="profile-select"
                value={profile}
                options={[
                  { value: "", label: "すべてのデータ" },
                  ...Object.keys(snapshot.config.profiles).map((value) => ({
                    value,
                    label: value,
                  })),
                ]}
                onChange={setProfile}
              />
              <Button
                icon={<PlayCircleOutlined />}
                loading={building}
                onClick={build}
              >
                Build
              </Button>
              <Button
                icon={<RocketOutlined />}
                onClick={() => setBuildOpen(true)}
              >
                Publish
              </Button>
              <Button
                onClick={() => setBuildOpen(true)}
                aria-label="生成結果を表示"
              >
                生成結果
              </Button>
            </Space>
          </div>
          <div className="workspace">
            <Explorer
              snapshot={snapshot}
              saved={state.saved}
              active={active}
              onSelect={select}
              fail={fail}
            />
            <main className="main-area">
              <div className="file-tabs">
                <Tabs
                  type="editable-card"
                  hideAdd
                  size="small"
                  activeKey={active}
                  items={opened
                    .filter((id) => snapshot.sources.some((s) => s.id === id))
                    .map((id) => {
                      const s = snapshot.sources.find((s) => s.id === id)!;
                      return {
                        key: id,
                        label: (
                          <span className="file-tab-label">
                            <SourceIcon source={s} />
                            {s.path
                              .split("/")
                              .at(-1)
                              ?.replace(/\.ya?ml$/, "")}
                            {session.dirtySource(s) && (
                              <span className="dirty-dot" />
                            )}
                          </span>
                        ),
                      };
                    })}
                  onChange={select}
                  onEdit={(key, action) => {
                    if (action !== "remove") return;
                    const remaining = opened.filter((id) => id !== key);
                    setOpened(remaining);
                    if (active === key) setActive(remaining.at(-1) ?? "");
                  }}
                />
              </div>
              {source ? (
                <>
                  <div className="editor-heading">
                    <div className="editor-title">
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
                      <Tag>
                        {source.data?.kind === "type"
                          ? typeKind(source)
                          : "TABLE"}
                      </Tag>
                    </div>
                    <Typography.Text
                      type="secondary"
                      className="source-location"
                    >
                      {source.path}
                    </Typography.Text>
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
            <span className="welcome-eyebrow">YOUR DATA, YOUR WORLD</span>
            <Typography.Title>
              データから、
              <br />
              <span>世界をつくる。</span>
            </Typography.Title>
            <Typography.Paragraph>
              アイテムも、クエストも、そのつながりも。
              <br />
              構造を理解するエディタで、思いついたその場から。
            </Typography.Paragraph>
            <Space size="middle">
              <Button
                type="primary"
                size="large"
                icon={<FolderOpenOutlined />}
                onClick={open}
              >
                プロジェクトを開く
              </Button>
              <Button
                size="large"
                icon={<PlusOutlined />}
                onClick={() => setNewOpen(true)}
              >
                新しく作る
              </Button>
            </Space>
            <button className="sample-link" onClick={sample}>
              <ExperimentOutlined /> サンプルで試す <ArrowRightOutlined />
            </button>
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
              <strong>冒険の道具箱</strong>
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
              <CheckCircleOutlined /> つながりまで、見える。
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
            現在の編集内容から成果物を作成します。保存は別の操作です。
          </Typography.Paragraph>
          <Space>
            <Tag color="blue">{profile || "すべてのデータ"}</Tag>
            <Tag>編集 rev.{revision}</Tag>
            {dirty && <Tag color="gold">未保存の変更を含む</Tag>}
          </Space>
          <Button
            type="primary"
            size="large"
            icon={<PlayCircleOutlined />}
            loading={building}
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
            <Alert
              type="success"
              title={`Publish完了 · rev.${publishResult.revision} · ${publishResult.profile || "すべてのデータ"}`}
              description={publishResult.message}
              showIcon
            />
          )}
          {buildError && (
            <Alert
              type="error"
              showIcon
              title="処理を完了できませんでした"
              description={<pre className="build-error">{buildError}</pre>}
            />
          )}
          {buildResult && (
            <>
              <Result
                status="success"
                title="成果物を生成しました"
                subTitle={`${buildResult.tables} テーブル · ${buildResult.records} レコード · ${buildResult.profile || "すべてのデータ"} · rev.${buildResult.revision}`}
              />
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
                    !snapshot.config.publish.csharp &&
                    !snapshot.config.publish.binary
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
    </div>
  );
}
export default function Root() {
  const [dark, setDarkState] = useState(
    localStorage.getItem("theme") === "dark",
  );
  const [reduced, setReducedState] = useState(
    localStorage.getItem("reduced-motion") === "true" ||
      matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  const setDark = (value: boolean) => {
    setDarkState(value);
    localStorage.setItem("theme", value ? "dark" : "light");
  };
  const setReduced = (value: boolean) => {
    setReducedState(value);
    localStorage.setItem("reduced-motion", String(value));
  };
  useEffect(() => {
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    document.documentElement.dataset.reducedMotion = String(reduced);
  }, [dark, reduced]);
  return (
    <ConfigProvider
      locale={jaJP}
      theme={{
        algorithm: dark ? antTheme.darkAlgorithm : antTheme.defaultAlgorithm,
        token: {
          colorPrimary: "#5a67d8",
          borderRadius: 7,
          fontFamily:
            '-apple-system, BlinkMacSystemFont, "Hiragino Sans", "Noto Sans JP", sans-serif',
          fontSize: 13,
          // Changing AntD's motion token inserts a provider on its first toggle,
          // remounting Workspace and reloading the project. Keep this tree stable;
          // MotionConfig and data-reduced-motion control interpolation instead.
          motion: true,
          motionDurationFast: ".08s",
          motionDurationMid: ".12s",
          motionDurationSlow: ".16s",
          motionEaseOut: easeOutExpoCss,
          motionEaseInOut: easeOutExpoCss,
        },
        components: {
          Table: { cellPaddingBlockSM: 5, cellPaddingInlineSM: 8 },
          Button: {
            defaultShadow: "none",
            primaryShadow: "0 3px 10px #5a67d824",
          },
          Tree: {
            titleHeight: 28,
            indentSize: 16,
            nodeSelectedBg: dark ? "#262d50" : "#e9edff",
            motionDurationSlow: ".1s",
          },
        },
      }}
    >
      <MotionConfig reducedMotion={reduced ? "always" : "user"}>
        <AntApp>
          <Workspace
            dark={dark}
            setDark={setDark}
            reduced={reduced}
            setReduced={setReduced}
          />
        </AntApp>
      </MotionConfig>
    </ConfigProvider>
  );
}
