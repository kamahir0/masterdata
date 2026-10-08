import { uiMessage } from "./language";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { Alert, AutoComplete, Button, Drawer, Empty, Input, Space, Spin, Tooltip, Typography, type InputRef } from "antd";
import { DeleteOutlined, LeftOutlined, PlusOutlined, RightOutlined, TagsOutlined, WarningOutlined } from "@ant-design/icons";
import { useInteraction, useSurface } from "./app";
import { desktop, type TagView } from "./workspace";

type Typing = { authority: TagView; index: number | null; initial: string; text: string; composing: boolean };
export function TagPanel() {
  const target = useInteraction().tags, s = useSurface();
  const [view, setView] = useState<TagView | null>(null), [loading, setLoading] = useState(false),
    [error, setError] = useState<string | null>(null), [text, setText] = useState(""),
    [editing, setEditing] = useState<number | null>(null);
  const request = useRef(0), value = useRef(view), typing = useRef<Typing | null>(null),
    committing = useRef<Promise<boolean> | null>(null), input = useRef<InputRef>(null),
    body = useRef<HTMLDivElement>(null), focused = useRef<typeof target>(null);
  const restore = useRef<{target: typeof target; intent: number; index: number | null} | null>(null);
  value.current = view;
  const blocked = s.busy || s.pending || s.status.recoveryRequired || s.status.uncertain.includes(target?.source ?? "");
  const stale = !view || view.revision !== s.projection?.revision || view.generation !== s.projection?.generation;
  useEffect(() => {
    const mine = ++request.current;
    if (s.pending && s.externalPending) return;
    if (!target || s.pending || target.epoch !== s.status.epoch || target.source !== s.projection?.source) {
      setView(null); return;
    }
    setLoading(true); setError(null);
    void desktop.tagView(target).then(next => {
      if(mine !== request.current || desktop.interaction.tags !== target || next?.generation !== desktop.surface.projection?.generation) return;
      setView(next); setLoading(false);
    }).catch(e => {
      if(mine !== request.current) return;
      setError(e instanceof Error ? e.message : String(e)); setView(null); setLoading(false);
    });
    return () => { request.current++; };
  }, [target, s.projection?.revision, s.projection?.generation, s.pending, s.externalPending, s.status.epoch]);
  const cancel = useCallback(() => { typing.current = null; setText(""); setEditing(null); }, []);
  useEffect(cancel, [target?.source, target?.row, target?.epoch, cancel]);
  const apply = useCallback(async (authority: TagView, operation: Record<string, unknown>) => {
    const element = document.activeElement;
    restore.current = element && body.current?.contains(element) ? {target: desktop.interaction.tags, intent: desktop.inputIntent,
      index: typeof operation.index === "number" ? operation.index : null} : null;
    const ok = await desktop.tagEdit(authority, operation);
    if(ok) cancel();
    else restore.current = null;
    return ok;
  }, [cancel]);
  const commit = useCallback((explicit = false): Promise<boolean> => {
    if(committing.current) return committing.current;
    const current = typing.current;
    if(!current || (!explicit && current.text === current.initial)) return Promise.resolve(true);
    if(current.composing) return Promise.resolve(false);
    committing.current = apply(current.authority, current.index === null
      ? {kind: "add", text: current.text} : {kind: "replace", index: current.index, text: current.text})
      .finally(() => { committing.current = null; });
    return committing.current;
  }, [apply]);
  useLayoutEffect(() => {
    if(!target) return;
    return desktop.bindEditor(() => commit(), () => {
      const current = typing.current;
      return current ? {source: current.authority.source, revision: current.authority.revision, generation: current.authority.generation,
        label: "レコードのタグ", text: current.text, dirty: current.text !== current.initial, cancel} : null;
    });
  }, [target, commit, cancel]);
  useLayoutEffect(() => {
    const next = restore.current;
    if(next && (next.target !== target || next.intent !== desktop.inputIntent)) restore.current = null;
    const restoring = next?.target === target && next.intent === desktop.inputIntent;
    if(!target || !view || loading || stale || blocked || (!restoring && (focused.current === target || target.inputIntent !== desktop.inputIntent))) return;
    const control = restoring
      ? (next.index !== null ? body.current?.querySelector<HTMLElement>(`[data-tag-index="${next.index}"] .tag-entry-value`) : null) ?? body.current?.querySelector<HTMLElement>("input[aria-label=\"新しいタグ\"]")
      : body.current?.querySelector<HTMLElement>("input:not(:disabled),button:not(:disabled)");
    control?.focus({preventScroll: true});
    focused.current = target;
    restore.current = null;
  }, [target, view, loading, stale, blocked]);
  const close = async (escape = false) => {
    const current = desktop.interaction.tags;
    if(escape) cancel();
    if((escape || await commit()) && desktop.interaction.tags === current) {
      desktop.closeTags(); desktop.viewport?.focus({preventScroll: true});
    }
  };
  const begin = async (index: number) => {
    const current = desktop.interaction.tags, intent = desktop.inputIntent;
    if(!current || !(await commit()) || current !== desktop.interaction.tags) return;
    const next = await desktop.tagView(current);
    if(!next || current !== desktop.interaction.tags || intent !== desktop.inputIntent) return;
    const entry = next.entries.find(entry => entry.index === index);
    if(!entry) return;
    value.current = next; setView(next); setEditing(index); setText(entry.text);
    typing.current = {authority: next, index, initial: entry.text, text: entry.text, composing: false};
    requestAnimationFrame(() => {
      if(current === desktop.interaction.tags && intent === desktop.inputIntent && !desktop.surface.pending)
        input.current?.focus({cursor: "all", preventScroll: true});
    });
  };
  const remove = async (index: number) => {
    const current = desktop.interaction.tags;
    if(!current || !(await commit()) || current !== desktop.interaction.tags) return;
    const next = await desktop.tagView(current);
    if(next && current === desktop.interaction.tags) await apply(next, {kind: "remove", index});
  };
  const change = (text: string) => {
    const authority = value.current;
    if(!authority || !authority.editable || blocked) return;
    if(!typing.current) typing.current = {authority, index: null, initial: "", text: "", composing: false};
    typing.current.text = text; setText(text);
  };
  const add = () => {
    const authority = value.current;
    if(!authority || !authority.editable || blocked || stale) return;
    // Explicit Add accepts even the empty text. Temporary empty input from
    // merely opening this editor does not create a source operation.
    typing.current ??= {authority, index: null, initial: "", text: "", composing: false};
    void commit(true);
  };
  const keys = (e: React.KeyboardEvent) => {
    if(e.nativeEvent.isComposing || e.keyCode === 229) return;
    if(e.key === "Escape") { e.preventDefault(); e.stopPropagation(); void close(true); }
    else if(e.key === "Enter" && (e.target instanceof HTMLInputElement)) {
      e.preventDefault(); e.stopPropagation();
      if(editing === null) add(); else void commit(true);
    }
    else if((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z" && !(e.target instanceof HTMLInputElement)) {
      e.preventDefault(); e.stopPropagation(); void desktop.undo(e.shiftKey);
    }
  };
  const validTarget = !!target && target.epoch === s.status.epoch && target.source === s.projection?.source;
  return <Drawer open={validTarget} placement="right" size={380} mask={false} keyboard={false} onClose={() => void close()}
    title={<Space><TagsOutlined aria-hidden="true" />レコードのタグ</Space>} classNames={{body: "tag-panel-body"}}>
    <div ref={body} className="tag-panel" onKeyDown={keys} aria-busy={loading || s.busy}>
      {error && <Alert type="error" showIcon title={uiMessage(error)} />}
      {view?.reason && <Alert type="warning" showIcon title="タグは読み取り専用です" description={uiMessage(view.reason)} />}
      {loading && !view ? <Spin size="small" /> : view && <>
        <div className="tag-entry-list">
          {view.total === 0 && <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="タグなし" />}
          {view.entries.map(entry => <div key={entry.index} className="tag-entry" data-tag-index={entry.index}>
            {editing === entry.index ? <Input ref={input} aria-label={`タグ ${entry.index + 1}`} value={text} readOnly={blocked || stale}
              onChange={e => change(e.target.value)}
              onCompositionStart={() => { if(typing.current) typing.current.composing = true; }}
              onCompositionEnd={() => { if(typing.current) typing.current.composing = false; }}
              onBlur={() => { if(!desktop.surface.externalPending) void commit(); }} />
              : <Tooltip title={uiMessage(entry.reason)}><Button type="text" className="tag-entry-value" aria-label={`タグを編集: ${entry.index + 1}`} aria-invalid={!entry.valid}
                icon={entry.valid ? undefined : <WarningOutlined aria-hidden="true" />} disabled={!view.editable || blocked || stale} onClick={() => void begin(entry.index)}>
                {entry.text || <Typography.Text type="secondary">空のタグ</Typography.Text>}
              </Button></Tooltip>}
            <Tooltip title="タグを削除"><Button type="text" icon={<DeleteOutlined aria-hidden="true" />} aria-label={`タグを削除: ${entry.index + 1}`}
              disabled={!view.editable || blocked || stale} onClick={() => void remove(entry.index)} /></Tooltip>
          </div>)}
        </div>
        {view.total > 64 && <Space className="tag-paging"><Button aria-label="前のタグ" icon={<LeftOutlined aria-hidden="true" />} disabled={view.start === 0 || blocked} onClick={() => {void commit().then(ok => {if(ok && desktop.interaction.tags === target) desktop.tagsPage(Math.max(0, view.start - 64));});}} />
          <Typography.Text type="secondary">{view.start + 1}–{Math.min(view.start + 64, view.total)} / {view.total}</Typography.Text>
          <Button aria-label="次のタグ" icon={<RightOutlined aria-hidden="true" />} disabled={view.start + 64 >= view.total || blocked} onClick={() => {void commit().then(ok => {if(ok && desktop.interaction.tags === target) desktop.tagsPage(view.start + 64);});}} /></Space>}
        {editing === null && <Space.Compact className="tag-add">
          <AutoComplete aria-label="新しいタグ" value={text} options={view.known.map(tag => ({value: tag, label: tag || "空のタグ"}))}
            filterOption={(query, option) => String(option?.value).includes(query)} onChange={change} disabled={!view.editable}>
            <Input aria-label="新しいタグ" placeholder="タグを追加" readOnly={blocked || stale}
              onCompositionStart={() => {if(!typing.current) change(text); if(typing.current) typing.current.composing = true;}}
              onCompositionEnd={() => {if(typing.current) typing.current.composing = false;}} />
          </AutoComplete>
          <Button icon={<PlusOutlined aria-hidden="true" />} aria-label="タグを追加" disabled={!view.editable || blocked || stale} onClick={add} />
        </Space.Compact>}
        {view.partial && <Typography.Text type="secondary" className="tag-partial">候補は読込済みの範囲です</Typography.Text>}
      </>}
    </div>
  </Drawer>;
}
