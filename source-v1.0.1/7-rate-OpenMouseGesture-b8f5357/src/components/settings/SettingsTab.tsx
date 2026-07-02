import { useState, useEffect, useRef, useCallback } from "react";
import { useStore } from "../../store/useStore";
import * as api from "../../api/commands";
import type { Config, GestureTriggerButton } from "../../types";
import { confirm, message, open, save } from "@tauri-apps/plugin-dialog";
import "./SettingsTab.css";

const triggerButtonOptions: Array<{ value: GestureTriggerButton; label: string }> = [
  { value: "right", label: "Right" },
  { value: "middle", label: "Middle" },
  { value: "x1", label: "XBUTTON1" },
  { value: "x2", label: "XBUTTON2" },
];

interface TriggerSettingProps {
  title: string;
  button: GestureTriggerButton;
  color: string;
  onButtonChange: (button: GestureTriggerButton) => void;
  onColorChange: (color: string) => void;
}

function TriggerSettingRow({
  title,
  button,
  color,
  onButtonChange,
  onColorChange,
}: TriggerSettingProps) {
  return (
    <div className="trigger-setting-row">
      <div className="trigger-setting-title">{title}</div>
      <label className="trigger-setting-field">
        <span>開始ボタン</span>
        <select value={button} onChange={(e) => onButtonChange(e.target.value as GestureTriggerButton)}>
          {triggerButtonOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
      <label className="trigger-setting-field">
        <span>軌跡色</span>
        <div className="trigger-color-field">
          <input type="color" value={color} onChange={(e) => onColorChange(e.target.value)} />
          <code>{color.toUpperCase()}</code>
        </div>
      </label>
    </div>
  );
}

export function SettingsTab() {
  const { config, setConfig, setGestures, pushHistory } = useStore();

  const [trajectory, setTrajectory] = useState(config.trajectory);
  const [ignoreExe, setIgnoreExe] = useState(config.ignore_exe.join("\n"));
  const [triggerA, setTriggerA] = useState(config.triggerA);
  const [triggerB, setTriggerB] = useState(config.triggerB);
  const [triggerC, setTriggerC] = useState(config.triggerC);
  const [triggerAColor, setTriggerAColor] = useState(config.triggerAColor);
  const [triggerBColor, setTriggerBColor] = useState(config.triggerBColor);
  const [triggerCColor, setTriggerCColor] = useState(config.triggerCColor);
  const [error, setError] = useState<string | null>(null);
  const skipSyncRef = useRef(false);

  useEffect(() => {
    if (skipSyncRef.current) {
      skipSyncRef.current = false;
      return;
    }
    setTrajectory(config.trajectory);
    setIgnoreExe(config.ignore_exe.join("\n"));
    setTriggerA(config.triggerA);
    setTriggerB(config.triggerB);
    setTriggerC(config.triggerC);
    setTriggerAColor(config.triggerAColor);
    setTriggerBColor(config.triggerBColor);
    setTriggerCColor(config.triggerCColor);
  }, [config]);

  const sanitizeIgnoreExe = (value: string) =>
    value
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean);

  const hasConfigChanged = (next: Config, prev: Config) => JSON.stringify(next) !== JSON.stringify(prev);

  const persistConfig = useCallback(
    async (partial: Partial<Config>) => {
      const previousConfig = config;
      const nextConfig = { ...previousConfig, ...partial };

      if (!hasConfigChanged(nextConfig, previousConfig)) {
        return;
      }

      setError(null);
      skipSyncRef.current = true;
      setConfig(nextConfig);

      try {
        await api.saveConfig(nextConfig);
        pushHistory({
          type: "config",
          action: "update",
          data: nextConfig,
          previousData: previousConfig,
        });
      } catch (err) {
        setError(err instanceof Error ? err.message : "設定の保存に失敗しました");
        skipSyncRef.current = false;
        setConfig(previousConfig);
      }
    },
    [config, pushHistory, setConfig]
  );

  const reloadFromDisk = useCallback(async () => {
    const [nextConfig, nextGestures] = await Promise.all([api.getConfig(), api.getGestures()]);
    setConfig(nextConfig);
    setGestures(nextGestures);
  }, [setConfig, setGestures]);

  const handleExport = useCallback(async () => {
    setError(null);
    try {
      const targetPath = await save({
        title: "設定をエクスポート",
        defaultPath: "GestureHotkeyApp-settings.gha.json",
        filters: [
          { name: "GestureHotkeyApp Settings", extensions: ["json"] },
          { name: "JSON", extensions: ["json"] },
        ],
      });

      if (!targetPath) {
        return;
      }

      await api.exportSettingsBundle(targetPath);
      await message("設定をエクスポートしました。別PCへこのファイルを持っていけば復元に使えます。", {
        title: "エクスポート完了",
        kind: "info",
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : "設定のエクスポートに失敗しました");
    }
  }, []);

  const handleImport = useCallback(async () => {
    setError(null);
    try {
      const accepted = await confirm("現在の設定を上書きしてインポートします。続行しますか？", {
        title: "設定をインポート",
        kind: "warning",
      });

      if (!accepted) {
        return;
      }

      const selectedPath = await open({
        title: "設定ファイルを選択",
        multiple: false,
        filters: [
          { name: "GestureHotkeyApp Settings", extensions: ["json"] },
          { name: "JSON", extensions: ["json"] },
        ],
      });

      if (!selectedPath || Array.isArray(selectedPath)) {
        return;
      }

      await api.importSettingsBundle(selectedPath);
      await reloadFromDisk();
      await message("設定をインポートしました。現在の設定画面にも反映済みです。", {
        title: "インポート完了",
        kind: "info",
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : "設定のインポートに失敗しました");
    }
  }, [reloadFromDisk]);

  return (
    <div className="settings-tab">
      <div className="settings-content">
        <h2 className="settings-title">グローバル設定</h2>

        <div className="settings-section">
          <h3 className="section-title">表示設定</h3>
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={trajectory}
              onChange={(e) => {
                const checked = e.target.checked;
                setTrajectory(checked);
                void persistConfig({ trajectory: checked });
              }}
            />
            <span>軌跡を表示する</span>
          </label>
        </div>

        <div className="settings-section">
          <h3 className="section-title">トリガーボタン設定</h3>
          <p className="section-desc">
            Trigger A / B / C ごとに開始ボタンと軌跡色を設定します。
          </p>

          <TriggerSettingRow
            title="Trigger A"
            button={triggerA}
            color={triggerAColor}
            onButtonChange={(value) => {
              setTriggerA(value);
              void persistConfig({ triggerA: value });
            }}
            onColorChange={(value) => {
              setTriggerAColor(value);
              void persistConfig({ triggerAColor: value });
            }}
          />

          <TriggerSettingRow
            title="Trigger B"
            button={triggerB}
            color={triggerBColor}
            onButtonChange={(value) => {
              setTriggerB(value);
              void persistConfig({ triggerB: value });
            }}
            onColorChange={(value) => {
              setTriggerBColor(value);
              void persistConfig({ triggerBColor: value });
            }}
          />

          <TriggerSettingRow
            title="Trigger C"
            button={triggerC}
            color={triggerCColor}
            onButtonChange={(value) => {
              setTriggerC(value);
              void persistConfig({ triggerC: value });
            }}
            onColorChange={(value) => {
              setTriggerCColor(value);
              void persistConfig({ triggerCColor: value });
            }}
          />
        </div>

        <div className="settings-section">
          <h3 className="section-title">グローバル無視EXE</h3>
          <p className="section-desc">
            改行区切りで入力した実行ファイル上ではジェスチャーを無効化します。
          </p>
          <textarea
            value={ignoreExe}
            onChange={(e) => {
              const value = e.target.value;
              setIgnoreExe(value);
              void persistConfig({ ignore_exe: sanitizeIgnoreExe(value) });
            }}
            placeholder="notepad.exe&#10;explorer.exe"
            rows={6}
          />
        </div>

        <div className="settings-section">
          <h3 className="section-title">設定のバックアップ / 復元</h3>
          <p className="section-desc">
            Trigger A / B / C 設定、軌跡色、gesture 一覧、action 設定、ignore_exe をまとめて出力・読込します。
          </p>
          <div className="settings-actions">
            <button type="button" className="settings-action-button" onClick={handleExport}>
              設定をエクスポート
            </button>
            <button type="button" className="settings-action-button secondary" onClick={handleImport}>
              設定をインポート
            </button>
          </div>
        </div>

        {error && <p className="settings-error">{error}</p>}
      </div>
    </div>
  );
}
