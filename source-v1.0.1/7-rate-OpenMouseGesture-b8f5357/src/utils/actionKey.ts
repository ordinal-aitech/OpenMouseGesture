import type { Action, TriggerSlot } from "../types";

export const DEFAULT_TRIGGER_SLOT: TriggerSlot = "A";

export function normalizeTriggerSlot(slot?: string | null): TriggerSlot {
  if (slot === "B" || slot === "C") {
    return slot;
  }
  return DEFAULT_TRIGGER_SLOT;
}

export function isWheelAction(action: Action): boolean {
  return action.trigger_type === "wheel" || (!action.gesture && !!action.wheel_trigger);
}

export function getActionKey(action: Action): string {
  if (isWheelAction(action)) {
    return `wheel:${normalizeTriggerSlot(action.trigger_slot)}:${action.wheel_trigger ?? ""}`;
  }

  return `gesture:${normalizeTriggerSlot(action.trigger_slot)}:${action.gesture}`;
}

export function getActionDisplayTrigger(action: Action): string {
  if (isWheelAction(action)) {
    return `Trigger ${normalizeTriggerSlot(action.trigger_slot)} / ${action.wheel_trigger ?? ""}`;
  }

  return `Trigger ${normalizeTriggerSlot(action.trigger_slot)}`;
}

export function matchesActionKey(action: Action, key: string): boolean {
  return getActionKey(action) === key;
}
