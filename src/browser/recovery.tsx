import { useEffect, useState, useSyncExternalStore } from "react";
import { Alert, Button, Modal, Space, Typography } from "antd";
import { browserStatus, resolveRecovery, reauthorizeAccess } from "./runtime";
import { inspect } from "./project";

export function BrowserRecovery({ fail }: { fail: (error: unknown) => void }) {
  const status = useSyncExternalStore(
    browserStatus.subscribe,
    browserStatus.get,
  );
  const [rows, setRows] = useState<{ path: string; state: string }[]>([]);
  const [dismissed, setDismissed] = useState(false);
  const recovery = status.recovery;
  useEffect(() => {
    let cancelled = false;
    setRows([]);
    if (recovery)
      void inspect(recovery)
        .then((value) => {
          if (!cancelled) setRows(value);
        })
        .catch(fail);
    return () => {
      cancelled = true;
    };
  }, [recovery, status.busy]);
  useEffect(() => setDismissed(false), [recovery]);
  const resolve = async (adopt: boolean) => {
    try {
      await resolveRecovery(adopt);
    } catch (error) {
      fail(error);
    }
  };
  return (
    <>
      {status.notice && <Alert type="info" showIcon title={status.notice} />}
      {recovery && dismissed && (
        <Alert
          type="warning"
          showIcon
          title="中断した保存が残っています。入力は保持しています。"
          action={
            <Button onClick={() => setDismissed(false)}>復旧情報を開く</Button>
          }
        />
      )}
      <Modal
        title="中断した保存の復旧"
        open={!!recovery && !dismissed}
        closable={!status.busy}
        onCancel={() => {
          if (!status.busy) setDismissed(true);
        }}
        maskClosable={false}
        footer={
          <Space wrap>
            <Button
              disabled={status.busy}
              onClick={() => void reauthorizeAccess().catch(fail)}
            >
              フォルダのアクセスを再許可
            </Button>
            <Button
              onClick={() => {
                if (!recovery) return;
                const url = URL.createObjectURL(
                  new Blob([recovery.raw], { type: "application/json" }),
                );
                const link = document.createElement("a");
                link.href = url;
                link.download = "masterdata-recovery.json";
                link.click();
                setTimeout(() => URL.revokeObjectURL(url), 1000);
              }}
            >
              復旧JSONをダウンロード
            </Button>
            <Button
              disabled={!recovery?.journal || status.busy}
              loading={status.busy}
              onClick={() => void resolve(false)}
            >
              保存前へ復旧
            </Button>
            {rows.length > 0 &&
              !status.recoveryLive &&
              rows.every((row) => row.state === "保存予定") && (
                <Button
                  type="primary"
                  disabled={status.busy}
                  onClick={() => void resolve(true)}
                >
                  保存済み内容を採用
                </Button>
              )}
          </Space>
        }
      >
        <Typography.Paragraph>
          通常保存を停止しています。外部で変更された内容は上書きしません。権限を拒否した場合は「フォルダのアクセスを再許可」で同じフォルダを選択してください。
        </Typography.Paragraph>
        <Typography.Text code>{recovery?.path}</Typography.Text>
        <div style={{ maxHeight: "40vh", overflow: "auto", marginTop: 16 }}>
          {rows.map((row) => (
            <div key={row.path}>
              <Typography.Text code>{row.path}</Typography.Text> · {row.state}
            </div>
          ))}
        </div>
      </Modal>
    </>
  );
}
