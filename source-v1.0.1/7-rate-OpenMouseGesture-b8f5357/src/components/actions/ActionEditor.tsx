import { useState, useEffect, useRef } from "react";
import { Button } from "../common/Button";
import { GestureCanvas } from "../common/GestureCanvas";
import type { Action, GestureTemplate, TriggerSlot, WheelTrigger } from "../../types";
import { getActionKey, normalizeTriggerSlot } from "../../utils/actionKey";
import "./ActionEditor.css";

interface ActionEditorProps {
  action: Action | null;
  gestures: GestureTemplate[];
  actions: Action[];
  groupName?: string;
  isNew?: boolean;
  onChange: (action: Action) => void;
}

const modifierOptions = ["Ctrl", "Shift", "Alt", "Win"];
const triggerSlotOptions: TriggerSlot[] = ["A", "B", "C"];
const keyOptions = [
  ...Array.from({ length: 26 }, (_, i) => String.fromCharCode(65 + i)),
  ...Array.from({ length: 10 }, (_, i) => String(i)),
  ...Array.from({ length: 24 }, (_, i) => `F${i + 1}`),
  "Left",
  "Right",
  "Up",
  "Down",
  "Tab",
  "Escape",
  "Enter",
  "Space",
  "Backspace",
  "Delete",
  "Insert",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "Apps",
  "CapsLock",
  "PrintScreen",
  "VolumeUp",
  "VolumeDown",
  "VolumeMute",
  "MediaPlayPause",
  "MediaStop",
  "MediaNext",
  "MediaPrev",
];

const wheelTriggerOptions: { value: WheelTrigger; label: string }[] = [
  { value: "wheel_up", label: "ホイール上" },
  { value: "wheel_down", label: "ホイール下" },
];

export function ActionEditor({ action, gestures, actions, groupName, isNew = false, onChange }: ActionEditorProps) {
  const [name, setName] = useState(action?.name || "");
  const [triggerType, setTriggerType] = useState<"gesture" | "wheel">(action?.trigger_type || "gesture");
  const [triggerSlot, setTriggerSlot] = useState<TriggerSlot>(normalizeTriggerSlot(action?.trigger_slot));
  const [gesture, setGesture] = useState(action?.gesture || "");
  const [wheelTrigger, setWheelTrigger] = useState<WheelTrigger | "">(action?.wheel_trigger || "");
  const [actionType, setActionType] = useState<"keystroke" | "command" | "url" | "window_operation">(
    action?.action_type || "keystroke"
  );
  const [keystroke, setKeystroke] = useState(action?.keystroke || "");
  const [modifiers, setModifiers] = useState<string[]>(action?.modifiers || []);
  const [command, setCommand] = useState(action?.command || "");
  const [url, setUrl] = useState(action?.url || "");
  const [operation, setOperation] = useState<"minimize" | "maximize" | "close">(action?.operation || "minimize");
  const [ignoreExe, setIgnoreExe] = useState(action?.ignore_exe?.join("\n") || "");
  const [error, setError] = useState<string | null>(null);

  const actionJsonRef = useRef<string>("");

  useEffect(() => {
    const newActionJson = JSON.stringify(action);
    if (newActionJson !== actionJsonRef.current) {
      actionJsonRef.current = newActionJson;
      setName(action?.name || "");
      setTriggerType(action?.trigger_type || "gesture");
      setTriggerSlot(normalizeTriggerSlot(action?.trigger_slot));
      setGesture(action?.gesture || "");
      setWheelTrigger(action?.wheel_trigger || "");
      setActionType(action?.action_type || "keystroke");
      setKeystroke(action?.keystroke || "");
      setModifiers(action?.modifiers || []);
      setCommand(action?.command || "");
      setUrl(action?.url || "");
      setOperation(action?.operation || "minimize");
      setIgnoreExe(action?.ignore_exe?.join("\n") || "");
      setError(null);
    }
  }, [action]);

  const currentActionKey = action ? getActionKey(action) : null;
  const availableGestures = gestures.filter((g) => {
    const candidateKey = `gesture:${triggerSlot}:${g.name}`;
    return !actions.some((item) => getActionKey(item) === candidateKey && getActionKey(item) !== currentActionKey);
  });

  const selectedGestureData = triggerType === "gesture" && gesture ? gestures.find((g) => g.name === gesture) : null;

  const toggleModifier = (mod: string) => {
    setModifiers((prev) => (prev.includes(mod) ? prev.filter((m) => m !== mod) : [...prev, mod]));
  };

  const handleTriggerTypeChange = (newType: "gesture" | "wheel") => {
    setTriggerType(newType);
    if (newType === "wheel") {
      setGesture("");
      if (!wheelTrigger) {
        setWheelTrigger("wheel_up");
      }
    } else {
      setWheelTrigger("");
    }
  };

  const validateAndSave = () => {
    if (triggerType === "gesture" && !gesture) {
      setError("ジェスチャーを選択してください");
      return false;
    }

    if (triggerType === "wheel" && !wheelTrigger) {
      setError("ホイールトリガーを選択してください");
      return false;
    }

    if (actionType === "keystroke" && !keystroke) {
      setError("キーを選択してください");
      return false;
    }
    if (actionType === "command" && !command.trim()) {
      setError("コマンドを入力してください");
      return false;
    }
    if (actionType === "url" && !url.trim()) {
      setError("URLを入力してください");
      return false;
    }

    const nextAction: Action = {
      name: name.trim() || undefined,
      group_id: action?.group_id,
      trigger_type: triggerType,
      trigger_slot: triggerSlot,
      gesture: triggerType === "gesture" ? gesture : "",
      wheel_trigger: triggerType === "wheel" && wheelTrigger ? wheelTrigger : undefined,
      action_type: actionType,
      keystroke: actionType === "keystroke" ? keystroke : undefined,
      modifiers: actionType === "keystroke" && modifiers.length > 0 ? modifiers : undefined,
      command: actionType === "command" ? command.trim() : undefined,
      url: actionType === "url" ? url.trim() : undefined,
      operation: actionType === "window_operation" ? operation : undefined,
      ignore_exe: ignoreExe
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean),
    };

    const nextKey = getActionKey(nextAction);
    const duplicate = actions.some((item) => getActionKey(item) === nextKey && getActionKey(item) !== currentActionKey);
    if (duplicate) {
      setError("同じ trigger_slot + gesture の組み合わせは既に使用されています");
      return false;
    }

    setError(null);
    onChange(nextAction);
    return true;
  };

  useEffect(() => {
    if (!isNew && actionType) {
      const timer = setTimeout(() => {
        validateAndSave();
      }, 500);
      return () => clearTimeout(timer);
    }
  }, [name, triggerType, triggerSlot, gesture, wheelTrigger, actionType, keystroke, modifiers, command, url, operation, ignoreExe]);

  return (
    <div className="action-editor">
      <h3 className="editor-title">{isNew ? "新規アクション" : "アクション編集"}</h3>

      <div className="editor-form">
        <div className="form-group">
          <label htmlFor="action-name">アクション名</label>
          <input
            id="action-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="アクションの名前"
          />
        </div>

        <div className="form-group">
          <label>グループ</label>
          <div className="readonly-value">{groupName || "未分類"}</div>
        </div>

        <div className="section-divider" />

        <div className="form-group">
          <h3 className="editor-title">トリガー</h3>
          <label htmlFor="trigger-type">トリガー種類</label>
          <select id="trigger-type" value={triggerType} onChange={(e) => handleTriggerTypeChange(e.target.value as "gesture" | "wheel")}>
            <option value="gesture">ジェスチャー</option>
            <option value="wheel">ホイール</option>
          </select>
        </div>

        <div className="form-group">
          <label htmlFor="trigger-slot">Trigger Slot</label>
          <select id="trigger-slot" value={triggerSlot} onChange={(e) => setTriggerSlot(e.target.value as TriggerSlot)}>
            {triggerSlotOptions.map((slot) => (
              <option key={slot} value={slot}>
                Trigger {slot}
              </option>
            ))}
          </select>
        </div>

        {triggerType === "gesture" && (
          <div className="form-group">
            <label htmlFor="action-gesture">ジェスチャー</label>
            <div className="gesture-select-row">
              {selectedGestureData && (
                <div className="gesture-preview-small">
                  <GestureCanvas points={selectedGestureData.points} width={50} height={50} strokeWidth={2} />
                </div>
              )}
              <select id="action-gesture" value={gesture} onChange={(e) => setGesture(e.target.value)}>
                <option value="">選択してください</option>
                {availableGestures.map((g) => (
                  <option key={g.name} value={g.name}>
                    {g.name}
                  </option>
                ))}
              </select>
            </div>
          </div>
        )}

        {triggerType === "wheel" && (
          <div className="form-group">
            <label htmlFor="wheel-trigger">ホイールトリガー</label>
            <select id="wheel-trigger" value={wheelTrigger} onChange={(e) => setWheelTrigger(e.target.value as WheelTrigger)}>
              <option value="">選択してください</option>
              {wheelTriggerOptions.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </select>
          </div>
        )}

        <div className="section-divider" />

        <div className="form-group">
          <h3 className="editor-title">実行内容</h3>
          <label htmlFor="action-type">アクション種類</label>
          <select
            id="action-type"
            value={actionType}
            onChange={(e) => setActionType(e.target.value as "keystroke" | "command" | "url" | "window_operation")}
          >
            <option value="keystroke">ホットキー</option>
            <option value="command">コマンド</option>
            <option value="url">URL</option>
            <option value="window_operation">ウィンドウ操作</option>
          </select>
        </div>

        {actionType === "keystroke" && (
          <>
            <div className="form-group">
              <label>修飾キー</label>
              <div className="modifier-buttons">
                {modifierOptions.map((mod) => (
                  <button
                    key={mod}
                    type="button"
                    className={`modifier-btn ${modifiers.includes(mod) ? "active" : ""}`}
                    onClick={() => toggleModifier(mod)}
                  >
                    {mod}
                  </button>
                ))}
              </div>
            </div>
            <div className="form-group">
              <label htmlFor="action-key">キー</label>
              <select id="action-key" value={keystroke} onChange={(e) => setKeystroke(e.target.value)}>
                <option value="">選択してください</option>
                {keyOptions.map((key) => (
                  <option key={key} value={key}>
                    {key}
                  </option>
                ))}
              </select>
            </div>
          </>
        )}

        {actionType === "command" && (
          <div className="form-group">
            <label htmlFor="action-command">コマンド</label>
            <input
              id="action-command"
              type="text"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              placeholder="実行するコマンドまたはパス"
            />
          </div>
        )}

        {actionType === "url" && (
          <div className="form-group">
            <label htmlFor="action-url">URL</label>
            <input id="action-url" type="text" value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://example.com" />
          </div>
        )}

        {actionType === "window_operation" && (
          <div className="form-group">
            <label htmlFor="action-operation">操作</label>
            <select id="action-operation" value={operation} onChange={(e) => setOperation(e.target.value as "minimize" | "maximize" | "close")}>
              <option value="minimize">最小化</option>
              <option value="maximize">最大化</option>
              <option value="close">閉じる</option>
            </select>
          </div>
        )}

        <div className="section-divider" />

        <div className="form-group">
          <label htmlFor="ignore-exe">無視するEXE（改行区切り）</label>
          <textarea
            id="ignore-exe"
            value={ignoreExe}
            onChange={(e) => setIgnoreExe(e.target.value)}
            placeholder={"notepad.exe\nexplorer.exe"}
            rows={3}
          />
        </div>

        {error && <p className="form-error">{error}</p>}

        {isNew && (
          <div className="editor-actions">
            <Button variant="primary" onClick={validateAndSave}>
              保存
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
