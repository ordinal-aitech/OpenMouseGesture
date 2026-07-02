import { useMemo, useState } from "react";
import { ConfirmDialog } from "../common/Dialog";
import { ActionList } from "./ActionList";
import { ActionEditor } from "./ActionEditor";
import { useStore } from "../../store/useStore";
import * as api from "../../api/commands";
import type { Action, ActionGroup, Config } from "../../types";
import { getActionKey } from "../../utils/actionKey";
import "./ActionsTab.css";

const DEFAULT_GROUP_ID = "group-uncategorized";
const DEFAULT_GROUP_NAME = "未分類";

function ensureGroups(config: Config): ActionGroup[] {
  return config.groups?.length ? config.groups : [{ id: DEFAULT_GROUP_ID, name: DEFAULT_GROUP_NAME }];
}

function createGroupId(existingGroups: ActionGroup[]): string {
  const existingIds = new Set(existingGroups.map((group) => group.id));
  let index = existingGroups.length + 1;
  while (existingIds.has(`group-${index}`)) {
    index += 1;
  }
  return `group-${index}`;
}

function createGroupName(existingGroups: ActionGroup[]): string {
  const baseName = "新しいグループ";
  const existingNames = new Set(existingGroups.map((group) => group.name));
  if (!existingNames.has(baseName)) {
    return baseName;
  }

  let index = 2;
  while (existingNames.has(`${baseName} ${index}`)) {
    index += 1;
  }
  return `${baseName} ${index}`;
}

export function ActionsTab() {
  const {
    actions,
    gestures,
    config,
    selectedAction,
    setSelectedAction,
    addAction,
    updateAction,
    deleteAction,
    setConfig,
    pushHistory,
  } = useStore();

  const [isNew, setIsNew] = useState(false);
  const [draftGroupId, setDraftGroupId] = useState<string>(DEFAULT_GROUP_ID);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const groups = useMemo(() => ensureGroups(config), [config]);
  const selectedActionData = actions.find((a) => getActionKey(a) === selectedAction);
  const selectedGroupId = (isNew ? draftGroupId : selectedActionData?.group_id) || groups[0]?.id || DEFAULT_GROUP_ID;
  const selectedGroupName = groups.find((group) => group.id === selectedGroupId)?.name || DEFAULT_GROUP_NAME;

  const editorAction = isNew
    ? ({
        name: "",
        group_id: selectedGroupId,
        trigger_type: "gesture",
        trigger_slot: "A",
        gesture: "",
        action_type: "keystroke",
      } as Action)
    : selectedActionData || null;

  const persistConfig = async (nextConfig: Config) => {
    await api.saveConfig(nextConfig);
    setConfig(nextConfig);
  };

  const handleAddToGroup = (groupId: string) => {
    setSelectedAction(null);
    setDraftGroupId(groupId);
    setIsNew(true);
  };

  const handleSelect = (actionKey: string | null) => {
    setSelectedAction(actionKey);
    setIsNew(false);
  };

  const handleAddGroup = async () => {
    setError(null);
    try {
      const currentGroups = ensureGroups(config);
      const nextGroups = [
        ...currentGroups,
        {
          id: createGroupId(currentGroups),
          name: createGroupName(currentGroups),
        },
      ];

      await persistConfig({
        ...config,
        groups: nextGroups,
        actions,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleRenameGroup = async (groupId: string, nextName: string) => {
    const trimmedName = nextName.trim() || DEFAULT_GROUP_NAME;
    setError(null);

    try {
      const nextGroups = ensureGroups(config).map((group) =>
        group.id === groupId ? { ...group, name: trimmedName } : group
      );

      await persistConfig({
        ...config,
        groups: nextGroups,
        actions,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleChange = async (action: Action) => {
    setError(null);
    try {
      const nextAction = {
        ...action,
        group_id: action.group_id || selectedGroupId,
      };

      if (isNew) {
        await api.addAction(nextAction);
        addAction(nextAction);
        pushHistory({
          type: "action",
          action: "add",
          data: nextAction,
        });
        setIsNew(false);
        setSelectedAction(getActionKey(nextAction));
      } else if (selectedAction) {
        const oldData = actions.find((a) => getActionKey(a) === selectedAction);
        const newKey = getActionKey(nextAction);

        if (selectedAction !== newKey) {
          await api.deleteAction(selectedAction);
          await api.addAction(nextAction);
          deleteAction(selectedAction);
          addAction(nextAction);
        } else {
          await api.updateAction(selectedAction, nextAction);
          updateAction(selectedAction, nextAction);
        }

        pushHistory({
          type: "action",
          action: "update",
          data: nextAction,
          previousData: oldData,
        });
        setSelectedAction(newKey);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleDelete = async (actionKey: string) => {
    setError(null);
    try {
      const oldData = actions.find((a) => getActionKey(a) === actionKey);
      await api.deleteAction(actionKey);
      deleteAction(actionKey);
      pushHistory({
        type: "action",
        action: "delete",
        data: { key: actionKey },
        previousData: oldData,
      });
      if (selectedAction === actionKey) {
        setSelectedAction(null);
        setIsNew(false);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="actions-tab">
      {error && <p className="error-message">{error}</p>}

      <div className="actions-content">
        <div className="actions-list-panel">
          <ActionList
            actions={actions}
            groups={groups}
            gestures={gestures}
            selectedAction={selectedAction}
            onSelect={handleSelect}
            onDelete={handleDelete}
            onAddToGroup={handleAddToGroup}
            onAddGroup={handleAddGroup}
            onRenameGroup={handleRenameGroup}
          />
        </div>

        <div className="actions-editor-panel">
          {selectedAction || isNew ? (
            <ActionEditor
              action={editorAction}
              gestures={gestures}
              actions={actions}
              groupName={selectedGroupName}
              isNew={isNew}
              onChange={handleChange}
            />
          ) : (
            <div className="action-placeholder">
              <p>グループ配下のアクションを選択して編集するか、グループ行の「+」から新規追加してください。</p>
            </div>
          )}
        </div>
      </div>

      <ConfirmDialog
        open={showDeleteConfirm}
        title="アクションの削除"
        message={`「${selectedAction ?? ""}」を削除しますか？`}
        confirmLabel="削除"
        variant="danger"
        onConfirm={async () => {
          if (selectedAction) {
            await handleDelete(selectedAction);
          }
          setShowDeleteConfirm(false);
        }}
        onCancel={() => setShowDeleteConfirm(false)}
      />
    </div>
  );
}
