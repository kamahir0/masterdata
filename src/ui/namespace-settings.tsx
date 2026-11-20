import React, { useState } from "react";
import { Button, Card, Modal, Select, Switch } from "antd";
import type { Snapshot } from "../core/types";
import {
  defaultNamespaces,
  effectiveNamespace,
  folderChain,
  included,
  namespaceError,
  sourceFolder,
} from "../core/namespaces";
import { session } from "../core/session";
import { Entry } from "./inputs";

export function NamespaceSettings({ snapshot }: { snapshot: Snapshot }) {
  const ns = snapshot.config.namespaces ?? defaultNamespaces;
  const [selection, setSelection] = useState<string>();
  const [confirming, setConfirming] = useState(false);
  const folders = [
    ...new Set(
      [
        ...snapshot.folders,
        ...snapshot.sources.map((s) => sourceFolder(s.path)),
      ].flatMap(folderChain),
    ),
  ].sort();
  const folder =
    selection && folders.includes(selection) ? selection : folders[0];
  const chain = folder ? folderChain(folder) : [];
  const blocked = chain.slice(0, -1).filter((p) => !included(ns, p));
  const active = !!folder && !blocked.length && included(ns, folder);
  const update = (change: Partial<typeof ns>) =>
    session.change("C#の名前空間を編集", (s) => ({
      ...s,
      config: {
        ...s.config,
        namespaces: {
          ...(s.config.namespaces ?? defaultNamespaces),
          ...change,
        },
      },
    }));
  const error = folder && namespaceError(ns, folder);
  function toggle(on: boolean) {
    const paths = on ? [...blocked, folder!] : [folder!];
    const folders = {
      ...ns.folders,
      ...Object.fromEntries(paths.map((path) => [path, on])),
    };
    update({ folders });
    setConfirming(false);
  }
  return (
    <Card size="small" title="C#の名前空間">
      <label>ルート名前空間</label>
      <Entry
        label="ルート名前空間"
        value={ns.root}
        onCommit={(root) => {
          const error = namespaceError({ ...ns, root }, "");
          if (error) throw new Error(error);
          update({ root });
        }}
      />
      <label>
        <Switch
          aria-label="フォルダ構成を既定で反映"
          checked={ns.useFolders}
          onChange={(useFolders) => update({ useFolders })}
        />{" "}
        フォルダ構成を既定で反映
      </label>
      {folder && (
        <>
          <label>フォルダごとの設定</label>
          <Select
            aria-label="名前空間の対象フォルダ"
            style={{ width: "100%" }}
            showSearch
            value={folder}
            options={folders.map((value) => ({ value, label: value }))}
            onChange={setSelection}
          />
          <label>
            <Switch
              aria-label="このフォルダ名を名前空間に含める"
              checked={active}
              onChange={(on) =>
                on && blocked.length ? setConfirming(true) : toggle(on)
              }
            />{" "}
            このフォルダ名を名前空間に含める
          </label>
          {blocked.length > 0 && <div>「{blocked[0]}」がOFFのため無効</div>}
          <div>
            生成先 <code>{effectiveNamespace(ns, folder)}</code>
          </div>
          {error && <div role="alert">{error}</div>}
          {Object.hasOwn(ns.folders, folder) && (
            <Button
              size="small"
              onClick={() => {
                const folders = { ...ns.folders };
                delete folders[folder];
                update({ folders });
              }}
            >
              プロジェクトの既定に戻す
            </Button>
          )}
        </>
      )}
      <Modal
        title="親フォルダもONにしますか？"
        open={confirming}
        okText="まとめてONにする"
        cancelText="キャンセル"
        onCancel={() => setConfirming(false)}
        onOk={() => toggle(true)}
      >
        「{blocked.join("、")}
        」をONにすると、生成先は{" "}
        <code>{[ns.root, ...(folder?.split("/") ?? [])].join(".")}</code>{" "}
        になります。親を共有する他のフォルダにも反映されます。
      </Modal>
    </Card>
  );
}
