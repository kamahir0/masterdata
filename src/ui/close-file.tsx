import { useState } from "react";
import { Button, Modal, Space } from "antd";

export function CloseFileDialog({
  name,
  cancel,
  save,
  discard,
  fail,
}: {
  name: string;
  cancel: () => void;
  save: () => Promise<void>;
  discard: () => void;
  fail: (error: unknown) => void;
}) {
  const [busy, setBusy] = useState(false);
  return (
    <Modal
      open
      title="ファイルの変更を保存しますか？"
      onCancel={cancel}
      closable={!busy}
      maskClosable={false}
      keyboard={!busy}
      footer={
        <Space>
          <Button
            danger
            disabled={busy}
            onClick={() => {
              try {
                discard();
              } catch (error) {
                fail(error);
              }
            }}
          >
            保存しない
          </Button>
          <Button disabled={busy} onClick={cancel}>
            キャンセル
          </Button>
          <Button
            type="primary"
            aria-label="保存"
            loading={busy}
            onClick={async () => {
              setBusy(true);
              try {
                await save();
              } catch (error) {
                fail(error);
              } finally {
                setBusy(false);
              }
            }}
          >
            保存
          </Button>
        </Space>
      }
    >
      <p>「{name}」の変更を保存してから閉じますか？</p>
    </Modal>
  );
}
