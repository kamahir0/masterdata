import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useState,
  useSyncExternalStore,
  type CSSProperties,
} from "react";
import {
  App as AntApp,
  Alert,
  Badge,
  Button,
  ConfigProvider,
  Dropdown,
  Empty,
  Flex,
  Input,
  Modal,
  Radio,
  Select,
  Space,
  Spin,
  Tree,
  Typography,
  theme,
  type MenuProps,
  type TreeDataNode,
} from "antd";
import {
  CheckCircleOutlined,
  DatabaseOutlined,
  DesktopOutlined,
  FileOutlined,
  FolderOpenOutlined,
  MoreOutlined,
  MoonOutlined,
  ReloadOutlined,
  SaveOutlined,
  SearchOutlined,
  SunOutlined,
  UndoOutlined,
  RedoOutlined,
  WarningOutlined,
  PlusOutlined,
} from "@ant-design/icons";
import { desktop, basename, type Preference, type Surface } from "./workspace";
import { AuthoringGrid } from "./grid";
import { ComplexPanel } from "./complex";

export const useSurface = () =>
  useSyncExternalStore(desktop.subscribe, desktop.snapshot);
export const useInteraction = () =>
  useSyncExternalStore(
    desktop.subscribeInteraction,
    desktop.interactionSnapshot,
  );
function useMedia(query: string) {
  const [value, set] = useState(
    () => typeof matchMedia === "function" && matchMedia(query).matches,
  );
  useEffect(() => {
    if (typeof matchMedia !== "function") return;
    const media = matchMedia(query),
      changed = () => set(media.matches);
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  }, [query]);
  return value;
}
export function Application({ platform }: { platform: string }) {
  const s = useSurface(),
    systemDark = useMedia("(prefers-color-scheme: dark)"),
    reduced = useMedia("(prefers-reduced-motion: reduce)");
  const dark = s.theme === "dark" || (s.theme === "system" && systemDark);
  const appearance = useMemo(
    () => ({
      algorithm: [
        dark ? theme.darkAlgorithm : theme.defaultAlgorithm,
        theme.compactAlgorithm,
      ],
      cssVar: {
        key: `md-${dark ? "dark" : "light"}-${reduced ? "reduced" : "motion"}`,
      },
      token: {
        colorPrimary: "#26847e",
        colorInfo: "#26847e",
        colorBgLayout: dark ? "#11161c" : "#f1f3f5",
        colorBgContainer: dark ? "#191e23" : "#ffffff",
        colorBgElevated: dark ? "#22282e" : "#ffffff",
        borderRadius: 6,
        fontSize: 13,
        fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
        controlHeight: 28,
        motion: !reduced,
        motionUnit: 0.06,
        motionBase: 0,
        wireframe: false,
      },
      components: {
        Button: { paddingInline: 10 },
        Tree: { titleHeight: 30 },
        Modal: { borderRadiusLG: 10 },
        Drawer: { paddingLG: 16 },
      },
    }),
    [dark, reduced],
  );
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    document.documentElement.dataset.reducedMotion = String(reduced);
  }, [dark, reduced]);
  return (
    <ConfigProvider theme={appearance} componentSize="small">
      <AntApp className="desktop-app">
        <ThemeFrame platform={platform} s={s} />
      </AntApp>
    </ConfigProvider>
  );
}
function ThemeFrame({ platform, s }: { platform: string; s: Surface }) {
  const { token } = theme.useToken();
  const variables = {
    "--md-bg": token.colorBgContainer,
    "--md-app": token.colorBgLayout,
    "--md-panel": token.colorFillAlter,
    "--md-elevated": token.colorBgElevated,
    "--md-text": token.colorText,
    "--md-muted": token.colorTextSecondary,
    "--md-faint": token.colorTextTertiary,
    "--md-border": token.colorBorderSecondary,
    "--md-accent": token.colorPrimary,
    "--md-selected": token.colorPrimaryBg,
    "--md-focus": token.colorPrimaryBorder,
    "--md-error": token.colorError,
    "--md-error-bg": token.colorErrorBg,
    "--md-warning": token.colorWarning,
    "--md-shadow": token.boxShadowSecondary,
    "--md-radius": `${token.borderRadius}px`,
    "--md-font": token.fontFamily,
    "--md-motion": token.motionDurationMid,
    "--md-ease": token.motionEaseInOut,
    "--md-hover": token.colorFillTertiary,
  } as CSSProperties;
  const projectItems: MenuProps["items"] = [
    {
      key: "saveAll",
      label: "Save All",
      icon: <SaveOutlined />,
      disabled:
        !s.inventory ||
        s.busy ||
        s.status.recoveryRequired ||
        !!s.status.uncertain.length,
    },
    {
      key: "reload",
      label: "Reload Project",
      icon: <ReloadOutlined />,
      disabled: !s.inventory || s.busy,
    },
    { type: "divider" },
    {
      key: "appearance",
      label: "Application Settings",
      icon: <DesktopOutlined />,
    },
  ];
  return (
    <div
      className={`desktop-frame ${platform === "macos" ? "mac" : ""}`}
      style={variables}
      onPointerDownCapture={desktop.noteInputIntent}
      onKeyDownCapture={desktop.noteInputIntent}
    >
      <header id="titlebar">
        <div id="drag-region" data-tauri-drag-region>
          <DatabaseOutlined />
          <strong>MasterData</strong>
          <span id="project-name">{s.inventory?.project.name ?? ""}</span>
        </div>
        <nav aria-label="Project commands">
          <Space size={2}>
            <Button
              type="text"
              icon={<FolderOpenOutlined />}
              onClick={desktop.pickProject}
            >
              Open Project
            </Button>
            {s.inventory && (
              <>
                <Button
                  type="text"
                  icon={<SaveOutlined />}
                  disabled={
                    !s.projection ||
                    s.pending ||
                    s.busy ||
                    s.status.recoveryRequired ||
                    s.projection.writeStates.some(
                      (w) =>
                        w.outcome === "OutcomeUnknown" ||
                        w.outcome === "RecoveryRequired",
                    )
                  }
                  onClick={desktop.save}
                >
                  Save
                </Button>
                <Button
                  type="text"
                  icon={<CheckCircleOutlined />}
                  disabled={s.busy}
                  onClick={desktop.validate}
                >
                  Validate
                </Button>
              </>
            )}
            <Dropdown
              menu={{
                items: projectItems,
                onClick: ({ key }) => {
                  if (key === "saveAll") void desktop.saveAll();
                  else if (key === "reload")
                    void desktop.reloadProject().catch(desktop.showError);
                  else desktop.appearance(true);
                },
              }}
              trigger={["click"]}
            >
              <Button
                type="text"
                icon={<MoreOutlined />}
                aria-label="Project menu"
                title="Project menu"
              />
            </Dropdown>
          </Space>
        </nav>
      </header>
      <div id="workbench">
        {s.inventory && <Explorer s={s} />}
        <main>
          {!s.inventory ? (
            <section id="welcome">
              <DatabaseOutlined className="welcome-mark" />
              <Typography.Title level={3}>MasterData</Typography.Title>
              <Typography.Paragraph type="secondary">
                YAMLから、データを育てる。
              </Typography.Paragraph>
              <Button
                type="primary"
                icon={<FolderOpenOutlined />}
                onClick={desktop.pickProject}
              >
                Open Project
              </Button>
              {s.error && (
                <Alert
                  type="error"
                  showIcon
                  title="Projectを開けません"
                  description={s.error}
                />
              )}
            </section>
          ) : s.target ? (
            <TableSurface s={s} />
          ) : (
            <div className="select-source">
              <Empty
                image={Empty.PRESENTED_IMAGE_SIMPLE}
                description="Explorerからsourceを選択"
              />
            </div>
          )}
        </main>
      </div>
      <ChoiceModal s={s} />
      <CompareModal s={s} />
      <AppearanceModal s={s} />
    </div>
  );
}
function Explorer({ s }: { s: Surface }) {
  const [expanded, setExpanded] = useState(true);
  const nodes = useMemo(() => {
    const roots: TreeDataNode[] = [],
      folders = new Map<string, TreeDataNode>();
    for (const source of s.inventory?.sources ?? []) {
      let children = roots,
        path = "";
      for (const part of source.path.split("/").slice(0, -1)) {
        path = path ? `${path}/${part}` : part;
        let folder = folders.get(path);
        if (!folder) {
          folder = {
            key: `folder:${path}`,
            title: part,
            icon: <FolderOpenOutlined />,
            children: [],
            selectable: false,
          };
          folders.set(path, folder);
          children.push(folder);
        }
        children = folder.children!;
      }
      children.push({
        key: source.path,
        title: basename(source.path),
        icon: source.error ? <WarningOutlined /> : <FileOutlined />,
        isLeaf: true,
      });
    }
    return roots;
  }, [s.inventory]);
  return (
    <aside id="explorer">
      <div className="pane-heading">
        <Button
          type="text"
          size="small"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
        >
          Sources
        </Button>
      </div>
      {expanded && (
        <Tree
          treeData={nodes}
          blockNode
          showIcon
          defaultExpandAll
          selectedKeys={[s.target]}
          onSelect={(keys) => {
            if (keys[0])
              void desktop
                .selectTarget(String(keys[0]))
                .catch(desktop.showError);
          }}
          titleRender={(node) => (
            <span
              className="source"
              title={String(node.key)}
              data-path={node.key}
            >
              {String(node.title)}
              <span
                className="source-dirty"
                aria-label={
                  s.status.dirty.includes(String(node.key))
                    ? "未保存"
                    : undefined
                }
              >
                {s.status.dirty.includes(String(node.key)) ? "●" : ""}
              </span>
            </span>
          )}
        />
      )}
    </aside>
  );
}
function TableSurface({ s }: { s: Surface }) {
  const p = s.projection,
    history = desktop.history(),
    types = s.inventory?.types ?? [];
  const uncertain =
    !s.pending &&
    p?.writeStates.find(
      (w) => w.outcome === "OutcomeUnknown" || w.outcome === "RecoveryRequired",
    );
  const label = s.pending
    ? (s.inventory?.sources.find((v) => v.path === s.target)?.binding ??
      basename(s.target))
    : (p?.table.name ?? basename(s.target));
  const disabled =
    s.pending ||
    s.queryPending ||
    s.busy ||
    s.status.recoveryRequired ||
    !!uncertain;
  const items: MenuProps["items"] = [
    {
      key: "compare",
      label: "Compare save candidate",
      icon: <FileOutlined />,
      disabled: !p,
    },
  ];
  return (
    <section id="table-surface">
      <div id="table-context">
        <Typography.Text strong className="table-name">
          {label}
        </Typography.Text>
        <Typography.Text
          type="secondary"
          className="selection-label"
          title={s.target}
        >
          {s.pending ? s.target : (p?.source ?? s.target)}
        </Typography.Text>
        {!s.pending && p && p.sources.length > 1 && (
          <Select
            aria-label="Record source"
            className="source-picker"
            value={p.source}
            options={p.sources.map((source) => ({
              value: source,
              label: basename(source),
            }))}
            onChange={(path) =>
              void desktop.selectTarget(path).catch(desktop.showError)
            }
          />
        )}
        <span
          className="dirty-slot"
          aria-label={
            !s.pending && (p?.dirty || p?.schemaDirty) ? "未保存" : undefined
          }
        >
          {!s.pending && (p?.dirty || p?.schemaDirty) && (
            <Badge status="processing" title="未保存" />
          )}
        </span>
        <div className="context-actions">
          <Button
            type="text"
            icon={<UndoOutlined />}
            aria-label="Undo"
            title="Undo"
            disabled={disabled || !history.undo}
            onClick={() => void desktop.undo(false)}
          />
          <Button
            type="text"
            icon={<RedoOutlined />}
            aria-label="Redo"
            title="Redo"
            disabled={disabled || !history.redo}
            onClick={() => void desktop.undo(true)}
          />
          <Input
            id="search"
            className="source-search"
            prefix={<SearchOutlined />}
            placeholder="Find / Search"
            aria-label="Search current source"
            value={s.query}
            onChange={(e) => desktop.search(e.target.value)}
            disabled={s.pending || !p?.source}
            allowClear
          />
          <Dropdown
            trigger={["click"]}
            menu={{ items, onClick: () => void desktop.compare() }}
          >
            <Button
              type="text"
              icon={<MoreOutlined />}
              aria-label="Table actions"
              title="Table actions"
            />
          </Dropdown>
        </div>
      </div>
      <div id="grid-area">
        <AuthoringGrid
          projection={p}
          types={types}
          pending={s.pending || s.queryPending}
          busy={s.busy || s.status.recoveryRequired || !!uncertain}
        />
        {(s.pending || s.queryPending) && (
          <div id="pending" role="status">
            <Spin size="small" />
            <span>
              {basename(s.target)}
              {s.queryPending ? " · Search…" : " を開いています…"}
            </span>
          </div>
        )}
        {!s.pending && p && (
          <Button
            className="add-row"
            icon={<PlusOutlined />}
            onClick={() => void desktop.addRow()}
            disabled={disabled || !p.canAdd}
            title={p.addReason ?? "Add Row"}
          >
            Row
          </Button>
        )}
        {!s.pending && p?.conflict && (
          <Alert
            className="context-alert conflict-alert"
            type="warning"
            showIcon
            title="外部sourceが変更されています"
            description={
              <Space>
                <Button onClick={() => void desktop.compare()}>Compare</Button>
                <Button onClick={() => void desktop.reloadSource()}>
                  Reload source
                </Button>
              </Space>
            }
          />
        )}
        {uncertain && (
          <Alert
            className="context-alert"
            type="error"
            showIcon
            title={
              uncertain.outcome === "OutcomeUnknown"
                ? "保存結果を確認できません"
                : "Recovery Required"
            }
            description={
              <Space direction="vertical" size={4}>
                <span>
                  自動再試行を止めています。{uncertain.source}
                  を確認してください。
                </span>
                <Space>
                  <Button
                    onClick={() => void desktop.compare(uncertain.source)}
                  >
                    Compare
                  </Button>
                  <Button
                    onClick={() => void desktop.reloadSource(uncertain.source)}
                  >
                    Reload source…
                  </Button>
                </Space>
              </Space>
            }
          />
        )}
        {s.error && (
          <Alert
            className="context-alert"
            type="error"
            showIcon
            closable
            onClose={desktop.dismissError}
            title="操作を完了できません"
            description={<span className="error-detail">{s.error}</span>}
          />
        )}
        <ComplexPanel />
      </div>
      <ProblemsBar s={s} />
    </section>
  );
}
function ProblemsBar({ s }: { s: Surface }) {
  const { selection } = useInteraction(),
    p = s.projection;
  return (
    <>
      <div id="problems-bar">
        <Button
          type="text"
          size="small"
          icon={<WarningOutlined />}
          onClick={desktop.toggleProblems}
          aria-expanded={s.problemsOpen}
        >
          Problems{s.status.problemCount ? ` (${s.status.problemCount})` : ""}
        </Button>
        <span className="background-status" role="status">
          {s.status.diagnosticsPending ? "確認中…" : ""}
        </span>
        <span id="position" aria-live="polite">
          {!s.pending && p
            ? `${Math.min(selection.row + 1, p.totalRows)} / ${p.totalRows} · ${selection.field ?? ""}`
            : ""}
        </span>
      </div>
      <section
        id="problems"
        className={s.problemsOpen ? "problems-open" : ""}
        aria-label="Problems"
        aria-hidden={!s.problemsOpen}
        inert={!s.problemsOpen}
      >
        <div className="problems-heading">
          <Typography.Text strong>Problems</Typography.Text>
          <Button
            type="text"
            onClick={desktop.toggleProblems}
            aria-label="Close Problems"
          >
            Close
          </Button>
        </div>
        {s.problemsOpen &&
          (s.status.diagnosticsPending ? (
            <div className="quiet-empty">確認中…</div>
          ) : s.problems.length ? (
            s.problems.map((problem, i) => (
              <Button
                key={`${problem.generation}:${i}`}
                type="text"
                className="problem-item"
                icon={<WarningOutlined />}
                onClick={() => void desktop.focusProblem(problem)}
              >
                <Typography.Text type="secondary">
                  {basename(problem.source)}:{problem.line}
                </Typography.Text>
                <span>
                  {problem.code} · {problem.message}
                </span>
              </Button>
            ))
          ) : (
            <div className="quiet-empty">Problemsはありません。</div>
          ))}
      </section>
    </>
  );
}
function ChoiceModal({ s }: { s: Surface }) {
  const c = s.choice;
  return (
    <Modal
      title={c?.title}
      open={!!c}
      onCancel={() => c?.finish("Cancel")}
      destroyOnHidden
      footer={c?.actions.map((action, i) => (
        <Button
          key={action}
          autoFocus={i === c.actions.length - 1}
          type={i === 0 ? "primary" : "default"}
          danger={action === "Overwrite"}
          onClick={() => c.finish(action)}
        >
          {action}
        </Button>
      ))}
    >
      <Typography.Paragraph>{c?.description}</Typography.Paragraph>
    </Modal>
  );
}
function CompareModal({ s }: { s: Surface }) {
  const c = s.comparison,
    p = s.projection;
  const sources = p
    ? [...new Set([p.table.source, ...(p.source ? [p.source] : [])])]
    : [];
  return (
    <Modal
      title="Compare save candidate"
      open={!!c}
      onCancel={desktop.closeCompare}
      width="min(1000px, 94vw)"
      destroyOnHidden
      footer={
        <Space>
          <Button onClick={desktop.closeCompare}>Close</Button>
          {c?.conflict && (
            <Button danger onClick={desktop.overwrite}>
              Overwrite…
            </Button>
          )}
        </Space>
      }
    >
      {c && (
        <>
          <Select
            aria-label="Compare source"
            value={c.source}
            options={sources.map((source) => ({
              value: source,
              label: source,
            }))}
            onChange={(source) => void desktop.compare(source)}
            className="compare-source"
          />
          <div className="compare-panes">
            <label>
              現在のdisk
              <Input.TextArea
                aria-label="Current disk source"
                readOnly
                value={c.before}
              />
            </label>
            <label>
              保存候補
              <Input.TextArea
                aria-label="Save candidate"
                readOnly
                value={c.after}
              />
            </label>
          </div>
        </>
      )}
    </Modal>
  );
}
function AppearanceModal({ s }: { s: Surface }) {
  return (
    <Modal
      title="Application Settings"
      open={s.appearance}
      onCancel={() => desktop.appearance(false)}
      footer={
        <Button type="primary" onClick={() => desktop.appearance(false)}>
          Done
        </Button>
      }
      destroyOnHidden
    >
      <Flex vertical gap={12}>
        <Typography.Text strong>Appearance</Typography.Text>
        <Radio.Group
          value={s.theme}
          onChange={(e) => void desktop.setTheme(e.target.value as Preference)}
        >
          <Radio.Button value="system">
            <DesktopOutlined /> System
          </Radio.Button>
          <Radio.Button value="light">
            <SunOutlined /> Light
          </Radio.Button>
          <Radio.Button value="dark">
            <MoonOutlined /> Dark
          </Radio.Button>
        </Radio.Group>
        <Typography.Text type="secondary">
          Application全体に適用します。
        </Typography.Text>
      </Flex>
    </Modal>
  );
}
