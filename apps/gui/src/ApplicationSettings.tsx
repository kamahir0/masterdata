import { Modal, Radio, type RadioChangeEvent } from "antd";
import type { EffectiveTheme, ThemePreference } from "./theme";

type ApplicationSettingsModalProps = {
  open: boolean;
  onClose: () => void;
  themePreference: ThemePreference;
  effectiveTheme: EffectiveTheme;
  onThemePreferenceChange: (pref: ThemePreference) => void;
};

export function ApplicationSettingsModal({
  open,
  onClose,
  themePreference,
  effectiveTheme,
  onThemePreferenceChange,
}: ApplicationSettingsModalProps) {
  const handleChange = (e: RadioChangeEvent) => {
    onThemePreferenceChange(e.target.value as ThemePreference);
  };

  return (
    <Modal
      title="Application Settings"
      open={open}
      onCancel={onClose}
      footer={null}
      destroyOnHidden
      width={480}
    >
      <div className="app-settings-content">
        <section className="app-settings-section" aria-label="Appearance settings">
          <div className="app-settings-heading">
            <span className="dialog-kicker">USER PREFERENCES</span>
            <h3>Appearance</h3>
            <p>Choose your display theme. This preference applies across all projects on this machine.</p>
          </div>
          <div className="app-settings-control">
            <Radio.Group
              aria-label="Appearance"
              value={themePreference}
              onChange={handleChange}
            >
              <Radio value="system">
                System{themePreference === "system" ? ` (${effectiveTheme === "dark" ? "Dark" : "Light"})` : ""}
              </Radio>
              <Radio value="light">Light</Radio>
              <Radio value="dark">Dark</Radio>
            </Radio.Group>
          </div>
        </section>
      </div>
    </Modal>
  );
}
