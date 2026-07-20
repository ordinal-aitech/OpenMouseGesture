import { useEffect, useMemo, useState } from "react";
import { GestureCanvas } from "../common/GestureCanvas";
import type { Action, ActionGroup, GestureTemplate, WheelTrigger } from "../../types";
import { getActionKey, isWheelAction, normalizeTriggerSlot } from "../../utils/actionKey";
import "./ActionList.css";

interface ActionListProps {
  actions: Action[];
  groups: ActionGroup[];
  gestures: GestureTemplate[];
  selectedAction: string | null;
  onSelect: (gesture: string) => void;
  onDelete: (gesture: string) => void;
  onAddToGroup: (groupId: string) => void;
  onAddGroup: () => void;
  onRenameGroup: (groupId: string, nextName: string) => void;
}

const wheelTriggerLabels: Record<WheelTrigger, string> = {
  wheel_up: "ホイール上",
  wheel_down: "ホイール下",
};

const DEFAULT_GROUP_ID = "group-uncategorized";
const DEFAULT_GROUP_NAME = "未分類";

export function ActionList({
  actions,
  groups,
  gestures,
  selectedAction,
  onSelect,
  onDelete,
  onAddToGroup,
  onAddGroup,
  onRenameGroup,
}: ActionListProps) {
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>({});
  const [editingGroupId, setEditingGroupId] = useState<string | null>(null);
  const [editingGroupName, setEditingGroupName] = useState("");

  const normalizedGroups = useMemo(() => {
    const list = groups.length > 0 ? groups : [{ id: DEFAULT_GROUP_ID, name: DEFAULT_GROUP_NAME }];
    return list.map((group) => ({
      id: group.id || DEFAULT_GROUP_ID,
      name: group.name?.trim() || DEFAULT_GROUP_NAME,
    }));
  }, [groups]);

  const groupedActions = useMemo(() => {
    const groupMap = new Map(
      normalizedGroups.map((group) => [group.id, { group, items: [] as Action[] }])
    );

    for (const action of actions) {
      const groupId = action.group_id || DEFAULT_GROUP_ID;
      const entry = groupMap.get(groupId);
      if (entry) {
        entry.items.push(action);
      } else {
        const fallback = groupMap.get(DEFAULT_GROUP_ID);
        if (fallback) {
          fallback.items.push(action);
        } else {
          groupMap.set(DEFAULT_GROUP_ID, {
            group: { id: DEFAULT_GROUP_ID, name: DEFAULT_GROUP_NAME },
            items: [action],
          });
        }
      }
    }

    return Array.from(groupMap.values());
  }, [actions, normalizedGroups]);

  useEffect(() => {
    if (editingGroupId && !normalizedGroups.some((group) => group.id === editingGroupId)) {
      setEditingGroupId(null);
      setEditingGroupName("");
    }
  }, [editingGroupId, normalizedGroups]);

  const getGesturePoints = (gestureName: string): [number, number][] => {
    const gesture = gestures.find((g) => g.name === gestureName);
    return gesture?.points || [];
  };

  const getActionDescription = (action: Action): string => {
    switch (action.action_type) {
      case "keystroke": {
        const mods = action.modifiers?.join("+") || "";
        const key = action.keystroke || "";
        return mods ? `${mods}+${key}` : key;
      }
      case "command":
        return action.command || "";
      case "url":
        return action.url || "";
      case "window_operation": {
        const operationLabels: Record<string, string> = {
          minimize: "最小化",
          maximize: "最大化 / 元に戻す",
          close: "閉じる",
        };
        return operationLabels[action.operation || ""] || "";
      }
      case "text": {
        const raw = (action.text || "").replace(/\r\n|\r|\n/g, " ⏎ ");
        const maxLength = 30;
        return raw.length > maxLength ? `${raw.slice(0, maxLength)}…` : raw;
      }
      default:
        return action.action_type || "";
    }
  };

  const getGestureLabel = (action: Action): string => {
    if (isWheelAction(action)) {
      return action.wheel_trigger ? wheelTriggerLabels[action.wheel_trigger] : "ホイール";
    }

    return action.gesture || "未設定";
  };

  const toggleGroup = (groupId: string) => {
    setCollapsedGroups((prev) => ({
      ...prev,
      [groupId]: !prev[groupId],
    }));
  };

  const startRenameGroup = (group: ActionGroup) => {
    setEditingGroupId(group.id);
    setEditingGroupName(group.name);
  };

  const finishRenameGroup = () => {
    if (!editingGroupId) {
      return;
    }

    onRenameGroup(editingGroupId, editingGroupName);
    setEditingGroupId(null);
    setEditingGroupName("");
  };

  return (
    <div className="action-list">
      <div className="action-list-toolbar">
        <button type="button" className="add-group-button" onClick={onAddGroup}>
          + グループを追加
        </button>
      </div>

      <div className="action-list-header">
        <div className="action-col-name">アクション名</div>
        <div className="action-col-trigger">トリガー</div>
        <div className="action-col-gesture">ジェスチャー</div>
        <div className="action-col-action">内容</div>
      </div>

      {groupedActions.map(({ group, items }) => {
        const isCollapsed = collapsedGroups[group.id] ?? false;
        const isEditing = editingGroupId === group.id;

        return (
          <div key={group.id} className="action-group">
            <div className="action-group-header">
              <button
                type="button"
                className="action-group-toggle"
                onClick={() => toggleGroup(group.id)}
                aria-expanded={!isCollapsed}
              >
                <span className={`action-group-chevron ${isCollapsed ? "collapsed" : ""}`}>▾</span>
              </button>

              <div className="action-group-title-wrap">
                {isEditing ? (
                  <input
                    className="action-group-title-input"
                    value={editingGroupName}
                    onChange={(e) => setEditingGroupName(e.target.value)}
                    onBlur={finishRenameGroup}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        finishRenameGroup();
                      }
                      if (e.key === "Escape") {
                        setEditingGroupId(null);
                        setEditingGroupName("");
                      }
                    }}
                    autoFocus
                  />
                ) : (
                  <button type="button" className="action-group-title-button" onClick={() => startRenameGroup(group)}>
                    {group.name}
                  </button>
                )}
              </div>

              <span className="action-group-count">{items.length}</span>

              <button
                type="button"
                className="group-action-button"
                onClick={() => onAddToGroup(group.id)}
                title="このグループにアクションを追加"
              >
                +
              </button>
            </div>

            {!isCollapsed &&
              items.map((action) => {
                const actionKey = getActionKey(action);
                const wheel = isWheelAction(action);
                return (
                  <div
                    key={actionKey}
                    className={`action-item ${selectedAction === actionKey ? "selected" : ""}`}
                    onClick={() => onSelect(actionKey)}
                  >
                    <button
                      className="delete-button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDelete(actionKey);
                      }}
                      aria-label="削除"
                    >
                      ×
                    </button>
                    <div className="action-col-name">
                      <div className="action-name-primary">
                        {action.name && action.name.trim() ? action.name : "名称なし"}
                      </div>
                    </div>
                    <div className="action-col-trigger">
                      <div className={wheel ? "action-trigger-only wheel" : "action-trigger-only"}>
                        <div className={wheel ? "trigger-pill wheel" : "trigger-pill"}>
                          {`Trigger ${normalizeTriggerSlot(action.trigger_slot)}`}
                        </div>
                      </div>
                    </div>
                    <div className="action-col-gesture">
                      {wheel ? (
                        <div className="gesture-cell wheel">
                          <div className="gesture-name-text">
                            {action.wheel_trigger ? wheelTriggerLabels[action.wheel_trigger] : "ホイール"}
                          </div>
                        </div>
                      ) : (
                        <div className="gesture-cell">
                          <div className="gesture-preview">
                            <GestureCanvas
                              points={getGesturePoints(action.gesture)}
                              width={44}
                              height={44}
                              strokeWidth={2}
                            />
                          </div>
                          <div className="gesture-name-text">{getGestureLabel(action)}</div>
                        </div>
                      )}
                    </div>
                    <div className="action-col-action">
                      <div className="action-desc">{getActionDescription(action)}</div>
                    </div>
                  </div>
                );
              })}
          </div>
        );
      })}
    </div>
  );
}
