export interface GestureTemplate {
  name: string;
  points: [number, number][];
}

export type TriggerSlot = "A" | "B" | "C";
export type MouseTriggerButton = "left" | "right" | "middle" | "x1" | "x2";
export type TriggerModifier = "Ctrl" | "Alt" | "Shift";
export type GestureTrigger = string;
export type TriggerType = "gesture" | "wheel";
export type WheelTrigger = "wheel_up" | "wheel_down";

export interface ActionGroup {
  id: string;
  name: string;
}

export interface Action {
  name?: string;
  group_id?: string;
  trigger_type?: TriggerType;
  trigger_slot?: TriggerSlot;
  gesture: string;
  wheel_trigger?: WheelTrigger;
  action_type: "keystroke" | "command" | "url" | "window_operation" | "text";
  keystroke?: string;
  modifiers?: string[];
  command?: string;
  url?: string;
  operation?: "minimize" | "maximize" | "close";
  ignore_exe?: string[];
  /** `text` アクション専用: キャレット位置へそのまま入力するリテラル文字列。 */
  text?: string;
}

export interface Config {
  trajectory: boolean;
  ignore_exe: string[];
  triggerA: GestureTrigger;
  triggerB: GestureTrigger;
  triggerC: GestureTrigger;
  triggerAColor: string;
  triggerBColor: string;
  triggerCColor: string;
  groups: ActionGroup[];
  actions: Action[];
}

export type TabId = "gestures" | "actions" | "settings" | "licenses" | "info";

export interface HistoryEntry {
  type: "gesture" | "action" | "config";
  action: "add" | "update" | "delete";
  data: unknown;
  previousData?: unknown;
}
