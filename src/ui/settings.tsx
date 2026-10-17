import React from "react";
import { Button, Card, Drawer, Input, Select, Space, Typography } from "antd";
import { DeleteOutlined, PlusOutlined } from "@ant-design/icons";
import { type Snapshot } from "../core/types";
import { session } from "../core/session";
import { Entry } from "./inputs";
export function Settings({
  open,
  close,
  snapshot,
}: {
  open: boolean;
  close: () => void;
  snapshot: Snapshot;
}) {
  const config = snapshot.config;
  const update = (next: typeof config) =>
    session.change("プロジェクト設定を編集", (s) => ({ ...s, config: next }));
  const tags = [
    ...new Set(
      snapshot.sources.flatMap((s) =>
        Array.isArray(s.data?.records)
          ? s.data.records.flatMap((r: any) => r?.$tags ?? [])
          : [],
      ),
    ),
  ].map((value) => ({ value: String(value), label: String(value) }));
  return (
    <Drawer title="プロジェクト設定" width={540} open={open} onClose={close}>
      <div className="settings-content">
        <Card size="small" title="プロジェクト">
          <label>名前</label>
          <Entry
            label="プロジェクト名"
            value={config.name}
            onCommit={(name) => update({ ...config, name })}
          />
        </Card>
        <Card size="small" title="Publishの出力先">
          <Typography.Paragraph type="secondary">
            Projectからの相対パス、または絶対パスを指定できます。配置はPublishを押したときに行います。
          </Typography.Paragraph>
          <label>C#の生成ファイルを置くフォルダ</label>
          <Entry
            label="C#の出力先"
            value={config.publish.csharp}
            placeholder="../Unity/Assets/MasterData/Generated"
            onCommit={(csharp) =>
              update({ ...config, publish: { ...config.publish, csharp } })
            }
          />
          <label>バイナリの出力先ファイル</label>
          <Entry
            label="バイナリの出力先"
            value={config.publish.binary}
            placeholder="../Unity/Assets/StreamingAssets/masterdata.bytes"
            onCommit={(binary) =>
              update({ ...config, publish: { ...config.publish, binary } })
            }
          />
        </Card>
        <Card
          size="small"
          title="出力プロファイル"
          extra={
            <Button
              type="text"
              size="small"
              icon={<PlusOutlined />}
              onClick={() => {
                let i = 1;
                while (config.profiles[`profile${i}`]) i++;
                update({
                  ...config,
                  profiles: {
                    ...config.profiles,
                    [`profile${i}`]: { include: [], exclude: [] },
                  },
                });
              }}
            >
              追加
            </Button>
          }
        >
          {Object.entries(config.profiles).map(([name, profile]) => (
            <div className="profile-definition" key={name}>
              <Space.Compact>
                <Entry
                  label="プロファイル名"
                  value={name}
                  onCommit={(next) => {
                    if (!next || (next !== name && config.profiles[next]))
                      throw new Error("プロファイル名が空か重複しています。");
                    update({
                      ...config,
                      profiles: Object.fromEntries(
                        Object.entries(config.profiles).map(([n, p]) => [
                          n === name ? next : n,
                          p,
                        ]),
                      ),
                    });
                  }}
                />
                <Button
                  type="text"
                  danger
                  icon={<DeleteOutlined />}
                  aria-label={`${name}を削除`}
                  onClick={() =>
                    update({
                      ...config,
                      profiles: Object.fromEntries(
                        Object.entries(config.profiles).filter(
                          ([n]) => n !== name,
                        ),
                      ),
                    })
                  }
                />
              </Space.Compact>
              <label>含めるタグ（空ならすべて）</label>
              <Select
                mode="tags"
                aria-label={`${name}に含めるタグ`}
                options={tags}
                value={profile.include}
                onChange={(include) =>
                  update({
                    ...config,
                    profiles: {
                      ...config.profiles,
                      [name]: { ...profile, include },
                    },
                  })
                }
              />
              <label>除外するタグ</label>
              <Select
                mode="tags"
                aria-label={`${name}から除外するタグ`}
                options={tags}
                value={profile.exclude}
                onChange={(exclude) =>
                  update({
                    ...config,
                    profiles: {
                      ...config.profiles,
                      [name]: { ...profile, exclude },
                    },
                  })
                }
              />
            </div>
          ))}
        </Card>
      </div>
    </Drawer>
  );
}
