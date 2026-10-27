import { useState } from "react";
import { Input, Modal, Typography } from "antd";

export function NewProjectModal({
  open,
  browser,
  onCancel,
  onCreate,
}: {
  open: boolean;
  browser: boolean;
  onCancel: () => void;
  /** Chooses the destination and opens the project. Owner closes on success. */
  onCreate: (name: string) => Promise<void>;
}) {
  // The modal stays mounted, so the last name survives close / reopen.
  const [name, setName] = useState("新しいプロジェクト");
  return (
    <Modal
      title="新しいプロジェクト"
      open={open}
      okText="保存先を選択"
      cancelText="キャンセル"
      onCancel={onCancel}
      onOk={() => onCreate(name)}
    >
      <label>プロジェクト名</label>
      <Input
        aria-label="新しいプロジェクト名"
        autoFocus
        value={name}
        onChange={(e) => setName(e.target.value)}
      />
      <Typography.Paragraph
        type="secondary"
        style={{ marginTop: 12, marginBottom: 0 }}
      >
        {browser
          ? "次に選ぶフォルダ自体をプロジェクトとして初期化し、masterdata.toml と sources/ を追加します。既存のファイルは変更しません。"
          : "次に選ぶ保存先の中へ、この名前のフォルダを作成します。"}
      </Typography.Paragraph>
    </Modal>
  );
}
