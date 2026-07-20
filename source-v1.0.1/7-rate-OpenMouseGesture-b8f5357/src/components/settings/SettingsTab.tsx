import { useState, useEffect, useRef, useCallback } from "react";
import { useStore } from "../../store/useStore";
import * as api from "../../api/commands";
import type { Config, GestureTrigger, MouseTriggerButton, TriggerSlot } from "../../types";
import { confirm, message, open, save } from "@tauri-apps/plugin-dialog";
import "./SettingsTab.css";

const CAPTURE_ARM_DELAY_MS = 150;
const LEGACY_MOUSE_TRIGGERS = new Set(["left", "right", "middle", "x1", "x2"]);

const mouseButtonMap: Record<number, MouseTriggerButton | undefined> = {
  0: "left",
  1: "middle",
  2: "right",
  3: "x1",
  4: "x2",
};

const mouseButtonLabels: Record<MouseTriggerButton, string> = {
  left: "Mouse Left",
  right: "Mouse Right",
  middle: "Mouse Middle",
  x1: "Mouse X1",
  x2: "Mouse X2",
};

const keyLabelsByCode: Record<string, string> = {
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  Backquote: "`",
  Backslash: "\\",
  Backspace: "Backspace",
  BracketLeft: "[",
  BracketRight: "]",
  CapsLock: "CapsLock",
  Comma: ",",
  Delete: "Delete",
  End: "End",
  Enter: "Enter",
  Equal: "=",
  Escape: "Escape",
  Home: "Home",
  Insert: "Insert",
  Minus: "-",
  NumpadAdd: "Num +",
  NumpadDecimal: "Num .",
  NumpadDivide: "Num /",
  NumpadEnter: "Num Enter",
  NumpadMultiply: "Num *",
  NumpadSubtract: "Num -",
  PageDown: "PageDown",
  PageUp: "PageUp",
  Pause: "Pause",
  Period: ".",
  PrintScreen: "PrintScreen",
  Quote: "'",
  ScrollLock: "ScrollLock",
  Semicolon: ";",
  Slash: "/",
  Space: "Space",
  Tab: "Tab",
};

function normalizeMouseTrigger(trigger: string): MouseTriggerButton | null {
  const normalized = trigger.trim().toLowerCase();
  if (LEGACY_MOUSE_TRIGGERS.has(normalized)) {
    return normalized as MouseTriggerButton;
  }

  if (!normalized.startsWith("mouse:")) {
    return null;
  }

  const button = normalized.slice(6);
  return LEGACY_MOUSE_TRIGGERS.has(button) ? (button as MouseTriggerButton) : null;
}

function serializeMouseTrigger(button: MouseTriggerButton) {
  return `mouse:${button}`;
}

function serializeKeyboardTrigger(modifiers: string[], code: string) {
  const parts = [...modifiers, code];
  return `key:${parts.join("+")}`;
}

function getKeyboardKeyLabel(code: string, key: string) {
  if (code.startsWith("Key") && code.length === 4) {
    return code.slice(3);
  }

  if (code.startsWith("Digit") && code.length === 6) {
    return code.slice(5);
  }

  if (/^F\d{1,2}$/.test(code)) {
    return code;
  }

  if (code.startsWith("Numpad") && /^Numpad\d$/.test(code)) {
    return "Num " + code.slice(6);
  }

  if (keyLabelsByCode[code]) {
    return keyLabelsByCode[code];
  }

  const fallback = key.trim();
  if (!fallback || fallback === "Shift" || fallback === "Control" || fallback === "Alt" || fallback === "Meta") {
    return null;
  }

  return fallback.length === 1 ? fallback.toUpperCase() : fallback;
}

function formatTrigger(trigger: GestureTrigger) {
  if (!trigger.trim()) {
    return "Unassigned";
  }

  const mouseButton = normalizeMouseTrigger(trigger);
  if (mouseButton) {
    return mouseButtonLabels[mouseButton];
  }

  if (!trigger.startsWith("key:")) {
    return trigger || "未設定";
  }

  const payload = trigger.slice(4).split("+").filter(Boolean);
  if (payload.length === 0) {
    return "未設定";
  }

  const code = payload[payload.length - 1];
  const modifiers = payload.slice(0, -1);
  const keyLabel = getKeyboardKeyLabel(code, code) ?? code;
  return [...modifiers, keyLabel].join(" + ");
}

function isLeftMouseTrigger(trigger: GestureTrigger) {
  return normalizeMouseTrigger(trigger) === "left";
}

function buildKeyboardTrigger(event: KeyboardEvent): string | null {
  const code = event.code.trim();
  if (!code) {
    return null;
  }

  if (!getKeyboardKeyLabel(code, event.key)) {
    return null;
  }

  // New captures deliberately use one dedicated key. Existing modifier
  // combinations remain loadable for compatibility, but are not created here.
  return serializeKeyboardTrigger([], code);
}

interface TriggerSettingProps {
  title: string;
  trigger: GestureTrigger;
  color: string;
  isCapturing: boolean;
  duplicateOf: string | null;
  onStartCapture: () => void;
  onColorChange: (color: string) => void;
}

function TriggerSettingRow({
  title,
  trigger,
  color,
  isCapturing,
  duplicateOf,
  onStartCapture,
  onColorChange,
}: TriggerSettingProps) {
  return (
    <div className="trigger-setting-row">
      <div className="trigger-setting-title">{title}</div>
      <div className="trigger-setting-field trigger-setting-field-wide">
        <span>登録済みトリガー</span>
        <div className="trigger-capture-row">
          <code className="trigger-display">{formatTrigger(trigger)}</code>
          <button type="button" className={isCapturing ? "capture-button active" : "capture-button"} onClick={onStartCapture}>
            {isCapturing ? "入力待機中..." : "登録"}
          </button>
        </div>
        <p className="trigger-setting-hint">単一キーの利用を推奨します。ジェスチャー有効中、登録したキーボードキーは OpenMouseGesture 専用になり、通常の Windows／アプリ操作には使えません。Esc でキャンセルできます。</p>
        {isLeftMouseTrigger(trigger) && <p className="trigger-warning">Mouse Left は通常の左クリック操作と競合する可能性があります。</p>}
        {duplicateOf && <p className="trigger-warning">Trigger {duplicateOf} と同じ入力です。先に判定される Trigger が優先され、こちらは反応しません。</p>}
      </div>
      <label className="trigger-setting-field">
        <span>軌跡色</span>
        <div className="trigger-color-field">
          <input type="color" value={color} onChange={(event) => onColorChange(event.target.value)} />
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
  const [captureSlot, setCaptureSlot] = useState<TriggerSlot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [autostartEnabled, setAutostartEnabledState] = useState(false);
  const [autostartBusy, setAutostartBusy] = useState(false);
  const [autostartError, setAutostartError] = useState<string | null>(null);
  const skipSyncRef = useRef(false);
  const captureReadyAtRef = useRef(0);

  const reloadAutostartStatus = useCallback(async () => {
    try {
      const enabled = await api.getAutostartStatus();
      setAutostartEnabledState(enabled);
      setAutostartError(null);
    } catch (err) {
      setAutostartError(err instanceof Error ? err.message : "自動起動設定の取得に失敗しました");
    }
  }, []);

  useEffect(() => {
    void reloadAutostartStatus();
  }, [reloadAutostartStatus]);

  const handleAutostartToggle = useCallback(
    async (checked: boolean) => {
      setAutostartBusy(true);
      setAutostartError(null);
      try {
        const actual = await api.setAutostartEnabled(checked);
        setAutostartEnabledState(actual);
      } catch (err) {
        setAutostartError(err instanceof Error ? err.message : "自動起動設定の変更に失敗しました");
        // Never trust the attempted value on failure; always re-query the
        // real OS registration state so the checkbox cannot show a false
        // "success" that doesn't match reality.
        await reloadAutostartStatus();
      } finally {
        setAutostartBusy(false);
      }
    },
    [reloadAutostartStatus]
  );

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
      .map((entry) => entry.trim())
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

  const duplicateOfB = triggerB === triggerA ? "A" : null;
  const duplicateOfC = triggerC === triggerA ? "A" : triggerC === triggerB ? "B" : null;

  const reloadFromDisk = useCallback(async () => {
    const [nextConfig, nextGestures] = await Promise.all([api.getConfig(), api.getGestures()]);
    setConfig(nextConfig);
    setGestures(nextGestures);
  }, [setConfig, setGestures]);

  const applyCapturedTrigger = useCallback(
    (slot: TriggerSlot, trigger: GestureTrigger) => {
      const otherTriggers = slot === "A" ? [triggerB, triggerC] : slot === "B" ? [triggerA, triggerC] : [triggerA, triggerB];
      if (trigger.startsWith("key:") && !trigger.includes("+") && otherTriggers.includes(trigger)) {
        setError("That dedicated keyboard key is already assigned to another trigger.");
        return;
      }
      setCaptureSlot(null);
      switch (slot) {
        case "A":
          setTriggerA(trigger);
          void persistConfig({ triggerA: trigger });
          break;
        case "B":
          setTriggerB(trigger);
          void persistConfig({ triggerB: trigger });
          break;
        case "C":
          setTriggerC(trigger);
          void persistConfig({ triggerC: trigger });
          break;
      }
    },
    [persistConfig, triggerA, triggerB, triggerC]
  );

  useEffect(() => {
    if (!captureSlot) {
      return;
    }

    captureReadyAtRef.current = Date.now() + CAPTURE_ARM_DELAY_MS;

    const handleMouseCapture = (event: MouseEvent) => {
      if (Date.now() < captureReadyAtRef.current) {
        return;
      }

      const button = mouseButtonMap[event.button];
      if (!button) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();

      if (button === "left") {
        // Left click must never be registerable: it would swallow normal
        // left-click interaction everywhere and lock the app up.
        setError("Left click cannot be used as a trigger.");
        return;
      }

      applyCapturedTrigger(captureSlot, serializeMouseTrigger(button));
    };

    const handleKeyboardCapture = (event: KeyboardEvent) => {
      if (event.repeat) {
        event.preventDefault();
        return;
      }

      if (event.key === "Escape" && !event.ctrlKey && !event.altKey && !event.shiftKey) {
        event.preventDefault();
        setCaptureSlot(null);
        setError(null);
        return;
      }

      const trigger = buildKeyboardTrigger(event);
      if (!trigger) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();
      applyCapturedTrigger(captureSlot, trigger);
    };

    const suppressContextMenu = (event: Event) => {
      event.preventDefault();
    };

    window.addEventListener("mousedown", handleMouseCapture, true);
    window.addEventListener("keydown", handleKeyboardCapture, true);
    window.addEventListener("contextmenu", suppressContextMenu, true);

    return () => {
      window.removeEventListener("mousedown", handleMouseCapture, true);
      window.removeEventListener("keydown", handleKeyboardCapture, true);
      window.removeEventListener("contextmenu", suppressContextMenu, true);
    };
  }, [applyCapturedTrigger, captureSlot]);

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
          <h3 className="section-title">起動設定</h3>
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={autostartEnabled}
              disabled={autostartBusy}
              onChange={(event) => {
                void handleAutostartToggle(event.target.checked);
              }}
            />
            <span>Windows起動時に自動起動</span>
          </label>
          <p className="section-desc">
            有効にすると、Windowsサインイン時にトレイに常駐した状態で自動的に起動します。
          </p>
          {autostartError && <p className="settings-error">{autostartError}</p>}
        </div>

        <div className="settings-section">
          <h3 className="section-title">表示設定</h3>
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={trajectory}
              onChange={(event) => {
                const checked = event.target.checked;
                setTrajectory(checked);
                void persistConfig({ trajectory: checked });
              }}
            />
            <span>軌跡を表示する</span>
          </label>
        </div>

        <div className="settings-section">
          <h3 className="section-title">トリガー入力設定</h3>
          <p className="section-desc">
            Trigger A / B / C ごとに登録ボタンを押し、使いたいマウスボタンまたはキーボード入力をそのまま押して登録します。
          </p>
          <p className="section-desc section-desc-warning">
            キーボードトリガーは有効中に専用キーとして抑止されます。単一キーを推奨します。既存の修飾キー組合せは互換のため読み込めますが、新規登録では使用しません。
          </p>

          <TriggerSettingRow
            title="Trigger A"
            trigger={triggerA}
            color={triggerAColor}
            isCapturing={captureSlot === "A"}
            duplicateOf={null}
            onStartCapture={() => {
              setError(null);
              setCaptureSlot("A");
            }}
            onColorChange={(value) => {
              setTriggerAColor(value);
              void persistConfig({ triggerAColor: value });
            }}
          />

          <TriggerSettingRow
            title="Trigger B"
            trigger={triggerB}
            color={triggerBColor}
            isCapturing={captureSlot === "B"}
            duplicateOf={duplicateOfB}
            onStartCapture={() => {
              setError(null);
              setCaptureSlot("B");
            }}
            onColorChange={(value) => {
              setTriggerBColor(value);
              void persistConfig({ triggerBColor: value });
            }}
          />

          <TriggerSettingRow
            title="Trigger C"
            trigger={triggerC}
            color={triggerCColor}
            isCapturing={captureSlot === "C"}
            duplicateOf={duplicateOfC}
            onStartCapture={() => {
              setError(null);
              setCaptureSlot("C");
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
            onChange={(event) => {
              const value = event.target.value;
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
