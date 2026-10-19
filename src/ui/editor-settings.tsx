import { useEffect, useState } from "react";
import { SunOutlined, MoonOutlined, DesktopOutlined } from "@ant-design/icons";
import { Drawer, Radio, Switch, Typography } from "antd";

export interface EditorPreferences {
  theme: "light" | "dark" | "system";
  tabLayout: "scroll" | "wrap";
  explorer: boolean;
  explorerWidth: number;
  reduced: boolean;
}
export function useEditorPreferences() {
  const [preferences, setPreferences] = useState<EditorPreferences>(() => {
    let stored: Partial<EditorPreferences> = {};
    try {
      stored =
        JSON.parse(localStorage.getItem("editor-preferences") ?? "{}") ?? {};
    } catch {}
    return {
      theme: ["light", "dark", "system"].includes(stored.theme ?? "")
        ? stored.theme!
        : "system",
      tabLayout: stored.tabLayout === "wrap" ? "wrap" : "scroll",
      explorer: stored.explorer !== false,
      explorerWidth:
        typeof stored.explorerWidth === "number" &&
        Number.isFinite(stored.explorerWidth)
          ? Math.max(180, Math.min(480, stored.explorerWidth))
          : 236,
      reduced:
        typeof stored.reduced === "boolean"
          ? stored.reduced
          : localStorage.getItem("reduced-motion") === "true" ||
            matchMedia("(prefers-reduced-motion: reduce)").matches,
    };
  });
  const [systemDark, setSystemDark] = useState(
    () => matchMedia("(prefers-color-scheme: dark)").matches,
  );
  useEffect(() => {
    const query = matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(query.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  useEffect(() => {
    localStorage.setItem("editor-preferences", JSON.stringify(preferences));
  }, [preferences]);
  return {
    preferences,
    setPreferences,
    dark:
      preferences.theme === "dark" ||
      (preferences.theme === "system" && systemDark),
  };
}
export function EditorSettings({
  open,
  close,
  value,
  update,
}: {
  open: boolean;
  close: () => void;
  value: EditorPreferences;
  update: (value: EditorPreferences) => void;
}) {
  return (
    <Drawer title="エディター設定" width={460} open={open} onClose={close}>
      <div className="editor-preferences">
        <Typography.Paragraph type="secondary">
          このPCでの表示と操作を設定します。
        </Typography.Paragraph>
        <section>
          <h3 id="theme-setting">カラーテーマ</h3>
          <Radio.Group
            aria-labelledby="theme-setting"
            value={value.theme}
            onChange={(e) => update({ ...value, theme: e.target.value })}
            options={[
              {
                value: "light",
                label: (
                  <>
                    <SunOutlined aria-hidden /> ライト
                  </>
                ),
              },
              {
                value: "dark",
                label: (
                  <>
                    <MoonOutlined aria-hidden /> ダーク
                  </>
                ),
              },
              {
                value: "system",
                label: (
                  <>
                    <DesktopOutlined aria-hidden /> システム
                  </>
                ),
              },
            ]}
            optionType="button"
          />
        </section>
        <section>
          <h3 id="tabs-setting">ファイルタブ</h3>
          <Radio.Group
            aria-labelledby="tabs-setting"
            value={value.tabLayout}
            onChange={(e) => update({ ...value, tabLayout: e.target.value })}
            options={[
              { value: "scroll", label: "一行でスクロール" },
              { value: "wrap", label: "複数行に折り返す" },
            ]}
          />
        </section>
        <section className="preference-switch">
          <h3 id="motion-setting">モーション</h3>
          <Switch
            aria-labelledby="motion-setting"
            checked={!value.reduced}
            checkedChildren="ON"
            unCheckedChildren="OFF"
            onChange={(enabled) => update({ ...value, reduced: !enabled })}
          />
        </section>
      </div>
    </Drawer>
  );
}
