import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
  type CSSProperties,
  type ComponentRef,
} from "react";
import {
  App as AntApp,
  Alert,
  Badge,
  Button,
  ConfigProvider,
  Dropdown,
  Drawer,
  Empty,
  Flex,
  Input,
  Modal,
  Popover,
  Radio,
  Select,
  Space,
  Spin,
  Tree,
  Typography,
  Tag,
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
  CopyOutlined,
  EditOutlined,
} from "@ant-design/icons";
import { desktop, basename, type Preference, type Surface } from "./workspace";
import { AuthoringGrid } from "./grid";
import { ComplexPanel } from "./complex";
import { useCreation } from "./creation";
import { useSourcePath } from "./source-path";

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
  const [recoveryOpen,setRecoveryOpen]=useState(false);
  useEffect(()=>{if(s.status.recoveryRequired)void desktop.refreshInventory().catch(desktop.showError);},[s.status.recoveryRequired]);
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
            {s.status.recoveryRequired && <Button danger type="text" icon={<WarningOutlined/>} onClick={()=>setRecoveryOpen(true)}>Recovery Required</Button>}
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
      <RecoveryDrawer s={s} open={recoveryOpen && s.status.recoveryRequired} close={()=>setRecoveryOpen(false)} />
    </div>
  );
}
function Explorer({ s }: { s: Surface }) {
  const [expanded, setExpanded] = useState(true);
  const [keys,setKeys]=useState<string[]>(()=>s.inventory!.folders.map(p=>`folder:${p||"."}`));
  const [selected,setSelected]=useState(s.target);
  const [activeKey,setActiveKey]=useState<string|null>(null);
  const [focusRequest,setFocusRequest]=useState<{key:string;intent:number}|null>(null);
  const tree=useRef<ComponentRef<typeof Tree>>(null);
  const container=useRef<HTMLElement|null>(null);
  useEffect(()=>setSelected(s.target),[s.target]);
  const ancestors=(path:string)=>path.split("/").map((_,i,parts)=>`folder:${parts.slice(0,i+1).join("/")||"."}`);
  const creation=useCreation(s.inventory!,s.status.epoch,selected||s.target,async(path,folder,inputIntent)=>{
    if(inputIntent!==desktop.inputIntent)return;
    const key=folder?`folder:${path}`:path;
    setKeys(old=>[...new Set([...old,...ancestors(folder?path:path.split("/").slice(0,-1).join("/"))])]);
    setSelected(key);
    if(!folder)await desktop.selectTarget(path,"creation").catch(desktop.showError);
    if(inputIntent===desktop.inputIntent) {
      setActiveKey(key);
      setFocusRequest({key,intent:inputIntent});
    }
  });
  const creationPending=useRef(false);creationPending.current=!!creation.draft;
  const pathMutation=useSourcePath(s.status.epoch,(source,destination,inputIntent)=>{
    setKeys(old=>[...new Set([...old,...ancestors(destination.split("/").slice(0,-1).join("/"))])]);
    if(inputIntent!==desktop.inputIntent)return;
    setSelected(old=>old===source?destination:old);
    setActiveKey(destination);setFocusRequest({key:destination,intent:inputIntent});
  });
  const sourceItems:MenuProps={items:[{key:"move",label:"Rename / Move source",icon:<EditOutlined/>,disabled:!selected||selected.startsWith("folder:")||!!creation.draft||pathMutation.open||s.status.recoveryRequired||s.status.uncertain.includes(selected)}],onClick:()=>void pathMutation.begin(selected)};
  useLayoutEffect(()=>{
    if(!focusRequest)return;
    const frame=requestAnimationFrame(()=>{
      if(focusRequest.intent!==desktop.inputIntent||creationPending.current)return;
      tree.current?.scrollTo({key:focusRequest.key});
      // Ant Tree owns keyboard focus through aria-activedescendant. Its visual
      // node wrapper is not focusable; focus the tree with the accepted new key.
      container.current?.querySelector<HTMLElement>('[role="tree"]')?.focus();
    });
    return()=>cancelAnimationFrame(frame);
  },[focusRequest]);
  useEffect(()=>{
    if(creation.draft)setKeys(old=>[...new Set([...old,...ancestors(creation.draft!.folder)])]);
  },[creation.draft?.id]);
  const nodes = useMemo(() => {
    const roots: TreeDataNode[] = [],
      folders = new Map<string, TreeDataNode>();
    function ensureFolder(logical:string) {
      let children=roots,path="";
      for(const part of (logical||".").split("/")) {
        path=path?`${path}/${part}`:part;
        let folder=folders.get(path);
        if(!folder){folder={key:`folder:${path}`,title:part,icon:<FolderOpenOutlined/>,children:[]};folders.set(path,folder);children.push(folder);}
        children=folder.children!;
      }
      return children;
    }
    for(const folder of s.inventory?.folders??[])ensureFolder(folder);
    for (const source of s.inventory?.sources ?? []) {
      const parent=source.path.split("/").slice(0,-1).join("/");
      const children=parent?ensureFolder(parent):folders.get(".")?.children??roots;
      children.push({
        key: source.path,
        title: basename(source.path),
        icon: source.error ? <WarningOutlined /> : <FileOutlined />,
        isLeaf: true,
      });
    }
    if(creation.draft)ensureFolder(creation.draft.folder).push({key:"creation:temporary",title:creation.draft.filename,isLeaf:true,selectable:false});
    return roots;
  }, [s.inventory,creation.draft?.folder,creation.draft?.filename,creation.draft?.id]);
  return (
    <aside id="explorer" ref={container}>
      <div className="pane-heading">
        <Button
          type="text"
          size="small"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
        >
          Sources
        </Button>
        <Space size={0}>
          <Dropdown menu={creation.menu} trigger={["click"]}>
            <Button type="text" icon={<PlusOutlined/>} aria-label="New source artifact" disabled={!creation.ready||s.status.recoveryRequired||!!s.status.environmentError||!!creation.draft||pathMutation.open}>New</Button>
          </Dropdown>
          <Dropdown menu={sourceItems} trigger={["click"]}><Button type="text" icon={<MoreOutlined/>} aria-label="Source actions" title="Source actions"/></Dropdown>
        </Space>
      </div>
      {expanded && (
        <Dropdown menu={sourceItems} trigger={["contextMenu"]}>
        <Tree
          ref={tree}
          aria-label="Sources"
          activeKey={activeKey}
          onActiveChange={key=>setActiveKey(key===null?null:String(key))}
          treeData={nodes}
          blockNode
          showIcon
          expandedKeys={keys}
          onExpand={next=>setKeys(next.map(String))}
          selectedKeys={[selected]}
          onRightClick={({node})=>{const key=String(node.key);setSelected(key);setActiveKey(key);}}
          onKeyDown={event=>{if(event.key==="F2"&&!event.nativeEvent.isComposing){event.preventDefault();if(selected&&!selected.startsWith("folder:"))void pathMutation.begin(selected);}}}
          onSelect={(keys) => {
            if(keys[0]) {
              const key=String(keys[0]);setSelected(key);setActiveKey(key);
              if(!key.startsWith("folder:"))void desktop.selectTarget(key).catch(desktop.showError);
            }
          }}
          titleRender={(node) => node.key==="creation:temporary"?creation.inline:(
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
        </Dropdown>
      )}
      {creation.modal}
      {pathMutation.modal}
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
              {s.queryPending ? " · Search…" : s.externalPending ? " · 再確認中…" : " を開いています…"}
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
        {s.uncertainField && <Alert className="context-alert" type="warning" showIcon title="構造変更の結果を確認できません" description={<Button disabled={s.busy} onClick={()=>void desktop.recheckFieldOperation()}>Recheck actual source set</Button>} />}
        {s.heldInputs.length > 0 && <HeldInputs s={s} />}
        <ComplexPanel />
      </div>
      <ProblemsBar s={s} />
    </section>
  );
}
function HeldInputs({s}: {s: Surface}) {
  const [at, select] = useState(0);
  const index = Math.min(at, s.heldInputs.length - 1), input = s.heldInputs[index];
  return <Alert className="context-alert held-input-alert" type="warning" showIcon title={`未確定入力を${s.heldInputs.length}件保持しています`} description={<Popover trigger="click" title="変更後の対象を確認して貼り直してください" content={<Space direction="vertical" className="held-input-detail">
    {s.heldInputs.length > 1 && <Select aria-label="保持中の入力" value={index} options={s.heldInputs.map((input, at) => ({value: at, label: `${basename(input.source)} · ${input.label}`}))} onChange={select} />}
    <Typography.Text type="secondary">{input.source} · {input.label}</Typography.Text>
    <Input.TextArea aria-label="保持中の入力内容" value={input.text} readOnly rows={3} />
    <Typography.Text type="secondary">{input.reason}</Typography.Text>
    <Space><Button icon={<CopyOutlined />} onClick={() => void desktop.copyHeldInput(index)}>Copy</Button><Button danger onClick={() => desktop.discardHeldInput(index)}>この入力を破棄</Button></Space>
  </Space>}><Button size="small">入力を確認</Button></Popover>} />;
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
          danger={["Overwrite", "Delete", "Restore OLD"].includes(action)}
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
  const sources = c?.migration?.sources ?? (p
    ? [...new Set([p.table.source, ...(p.source ? [p.source] : [])])]
    : []);
  return (
    <Modal
      title={c?.migration ? "Compare structural change" : "Compare save candidate"}
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
            onChange={(source) => {if(c.migration)void desktop.migrationCompare(c.migration.token,source,c.migration.sources).catch(desktop.showError);else void desktop.compare(source);}}
            className="compare-source"
          />
          <div className="compare-panes">
            <label>
              {c.migration ? "レビューしたsource" : "現在のdisk"}
              <Input.TextArea
                aria-label="Current disk source"
                readOnly
                value={c.before}
              />
            </label>
            <label>
              {c.migration ? "変更候補" : "保存候補"}
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
function RecoveryDrawer({s,open,close}:{s:Surface;open:boolean;close:()=>void}) {
  const information=s.inventory?.recovery ?? [],[index,setIndex]=useState(0),info=information[Math.min(index,Math.max(0,information.length-1))];
  return <Drawer title="Migration Recovery Required" open={open} onClose={close} size={520} destroyOnHidden footer={<Space><Button disabled={s.busy || !info?.id} onClick={()=>{if(info)void desktop.recoverMigration(info.id,false);}}>Recheck actual source set</Button><Button danger disabled={s.busy || !info?.id} onClick={()=>{if(info)void desktop.recoverMigration(info.id,true);}}>Restore OLD…</Button></Space>}>
    <Space direction="vertical" size={12} className="recovery-detail">
      <Alert type="error" showIcon title="Source変更とBuildを停止しています" description="OLD／NEWの状態を確認してください。閉じてもgateは解除されません。"/>
      {information.length>1 && <Select aria-label="Recovery operation" value={index} options={information.map((record,i)=>({value:i,label:record.id || record.directory}))} onChange={setIndex}/>}
      <Typography.Paragraph>{info?.message}</Typography.Paragraph>
      {info?.files.map(file=><div className="recovery-file" key={file.source}><Space><Tag>{file.state}</Tag><Typography.Text strong>{file.source}</Typography.Text></Space><Typography.Paragraph type="secondary" copyable>{file.oldCopy}</Typography.Paragraph><Typography.Paragraph type="secondary" copyable>{file.newCopy}</Typography.Paragraph></div>)}
      <Typography.Text type="secondary" copyable>{info?.directory}</Typography.Text>
    </Space>
  </Drawer>;
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
