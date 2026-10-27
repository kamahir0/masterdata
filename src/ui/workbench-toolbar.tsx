import { Badge, Button, Dropdown, Space, Tooltip } from "antd";
import {
  CheckCircleOutlined,
  DownOutlined,
  FileDoneOutlined,
  FolderOpenOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  PlayCircleOutlined,
  RedoOutlined,
  RocketOutlined,
  SaveOutlined,
  UndoOutlined,
} from "@ant-design/icons";
import { SearchSelect } from "./search-select";

// Presentational only. Workspace owns save / guard / output timing, so every
// action is a callback captured from its latest render.
export function WorkbenchToolbar({
  browser,
  explorer,
  onToggleExplorer,
  onOpen,
  saving,
  canSave,
  onSave,
  onSaveAll,
  onReauthorize,
  dirty,
  undoLabel,
  canUndo,
  canRedo,
  onUndo,
  onRedo,
  profile,
  profiles,
  onProfileChange,
  building,
  publishing,
  buildError,
  onBuild,
  onShowBuild,
}: {
  browser: boolean;
  explorer: boolean;
  onToggleExplorer: () => void;
  onOpen: () => void;
  saving: boolean;
  /** A current file exists for the primary Save button. */
  canSave: boolean;
  onSave: () => void;
  onSaveAll: () => void;
  /** Browser-only folder permission recovery. */
  onReauthorize: () => void;
  dirty: boolean;
  /** Label of the newest undo entry, if any. */
  undoLabel?: string;
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
  profile: string;
  profiles: string[];
  onProfileChange: (profile: string) => void;
  building: boolean;
  publishing: boolean;
  buildError: string;
  onBuild: () => void;
  onShowBuild: () => void;
}) {
  return (
    <div className="app-toolbar">
      <Space size={4}>
        <Tooltip
          title={
            explorer ? "エクスプローラーを折りたたむ" : "エクスプローラーを表示"
          }
        >
          <Button
            type="text"
            aria-label="エクスプローラーの表示切替"
            aria-expanded={explorer}
            icon={explorer ? <MenuFoldOutlined /> : <MenuUnfoldOutlined />}
            onClick={onToggleExplorer}
          />
        </Tooltip>
        <Button icon={<FolderOpenOutlined />} onClick={onOpen}>
          開く
        </Button>
        <Space.Compact>
          <Button
            aria-label="保存"
            title="現在のファイルを保存"
            type="primary"
            icon={<SaveOutlined />}
            loading={saving}
            disabled={saving || !canSave}
            onClick={onSave}
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
                if (key === "access") onReauthorize();
                else onSaveAll();
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
        <Tooltip title={canUndo ? `元に戻す: ${undoLabel}` : "元に戻す"}>
          <Button
            type="text"
            icon={<UndoOutlined />}
            aria-label="元に戻す"
            disabled={!canUndo}
            onClick={onUndo}
          />
        </Tooltip>
        <Tooltip title="やり直す">
          <Button
            type="text"
            icon={<RedoOutlined />}
            aria-label="やり直す"
            disabled={!canRedo}
            onClick={onRedo}
          />
        </Tooltip>
        <Tooltip title={dirty ? "未保存の変更" : "保存済み"}>
          <span
            className="save-state"
            role="status"
            aria-label={dirty ? "未保存の変更" : "保存済み"}
          >
            {dirty ? <span className="dirty-dot" /> : <CheckCircleOutlined />}
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
              options={profiles.map((value) => ({ value, label: value }))}
              onChange={(value) => onProfileChange(value ?? "")}
            />
          </Tooltip>
          <Space.Compact className="build-controls">
            <Button
              icon={<PlayCircleOutlined />}
              loading={building}
              disabled={publishing}
              onClick={onBuild}
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
                onClick: onShowBuild,
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
  );
}
