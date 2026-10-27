import { Button, Space, Spin, Tag, Typography } from "antd";
import {
  ArrowRightOutlined,
  CheckCircleOutlined,
  ExperimentOutlined,
  FolderOpenOutlined,
  PlusOutlined,
} from "@ant-design/icons";
import { motion } from "motion/react";

export function WelcomeScreen({
  checking,
  browser,
  supported,
  recent,
  onOpenRecent,
  onOpen,
  onNew,
  onSample,
}: {
  /** Desktop initial project lookup is still pending. */
  checking: boolean;
  browser: boolean;
  /** Browser File System Access support. Ignored by desktop. */
  supported: boolean;
  recent?: { name: string };
  onOpenRecent: () => void;
  onOpen: () => void;
  onNew: () => void;
  onSample: () => void;
}) {
  if (checking)
    return (
      <div
        className="welcome"
        style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          height: "100%",
        }}
      >
        <Spin size="large" />
      </div>
    );
  return (
    <div className="welcome">
      {browser && (
        <span className="welcome-version" aria-label="アプリのバージョン">
          v{__APP_VERSION__}
        </span>
      )}
      <motion.div
        initial={{ opacity: 0, y: 18 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35 }}
        className="welcome-main"
      >
        <Typography.Title>MasterData</Typography.Title>
        <Typography.Paragraph>
          テーブルデータと型定義を統合管理するマスターデータエディタです。
          <br />
          プロジェクトを開くか、新規作成して編集を開始します。
        </Typography.Paragraph>
        <Space size="middle" wrap>
          {browser && recent && (
            <Button
              type="primary"
              size="large"
              icon={<FolderOpenOutlined />}
              onClick={onOpenRecent}
              disabled={!supported}
            >
              前回のプロジェクトを開く（{recent.name}）
            </Button>
          )}
          <Button
            type={browser && recent ? "default" : "primary"}
            size="large"
            icon={<FolderOpenOutlined />}
            onClick={onOpen}
            disabled={browser && !supported}
          >
            {browser && recent ? "別のプロジェクトを開く" : "プロジェクトを開く"}
          </Button>
          <Button
            size="large"
            icon={<PlusOutlined />}
            onClick={onNew}
            disabled={browser && !supported}
          >
            新しく作る
          </Button>
        </Space>
        {!browser && (
          <button className="sample-link" onClick={onSample}>
            <ExperimentOutlined /> サンプルで試す <ArrowRightOutlined />
          </button>
        )}
        {browser && (
          <Typography.Paragraph type="secondary" style={{ marginTop: 20 }}>
            {supported
              ? "ローカルのプロジェクトフォルダを選択します。変更は保存時にだけ反映されます。未保存の入力はタブ終了で失われます。"
              : "この環境では編集できません。デスクトップ版ChromeとHTTPS（またはlocalhost）で開いてください。"}
          </Typography.Paragraph>
        )}
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
          <strong>テーブル編集プレビュー</strong>
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
          <CheckCircleOutlined /> 外部キー参照・型定義・値の整合性を管理
        </div>
      </motion.div>
    </div>
  );
}
