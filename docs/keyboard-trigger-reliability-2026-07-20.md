# Keyboard Trigger Reliability Record

## Code-evidence diagnosis

`mouse_hook.rs` receives physical keyboard input through `WH_KEYBOARD_LL`.  Before this change it ignored only events carrying `LLKHF_INJECTED`, updated `PRESSED_KEYS`, started a keyboard session when `trigger_slot_for_keyboard_down` matched, and always returned `CallNextHookEx`.  Therefore a configured single key such as `CapsLock` or `F1` was both a gesture trigger and ordinary Windows/application input.

The old keyup path removed the key from `PRESSED_KEYS`, evaluated `keyboard_trigger_active`, and immediately called `complete_gesture`.  `complete_gesture` snapshots the current trajectory, recognizes it, dispatches its action, and clears the session.  A transient keyup between a left segment and an up segment therefore dispatched the shorter left gesture before a following keydown could arrive.

Mouse vendor software that maps a button to a normal physical keyboard event follows this same keyboard-hook path.  Application action injection follows `SendInput`; those events carry `LLKHF_INJECTED` and continue to bypass trigger tracking.  Mouse hook injection is separately filtered with `LLMHF_INJECTED`.

## New event/state sequence

For a modifier-free `key:<Code>` binding, `dedicated_keyboard_slot_for_vk` selects its unique slot.  Physical initial down and repeat down return `LRESULT(1)`, so Windows does not receive them.  The first down begins one gesture and records the dedicated key; repeat down cannot begin another.  Physical up is also consumed.

The first up sets `KeyboardReleaseState.release_pending` and schedules the centralized `TRIGGER_RELEASE_GRACE_MS` confirmation window (120 ms).  Trajectory collection remains active during this interval.  A same-key down cancels pending release and keeps the same trajectory/session.  The scheduled callback checks its generation, `PRESSED_KEYS`, live `GetAsyncKeyState`, gesture enablement, and the active slot before it can take the pending state and call `complete_gesture`; stale callbacks and duplicate up events do nothing.

Changing trigger settings, disabling gestures, hook uninstall/shutdown, and cancellation clear the keyboard release state without dispatching.  Wheel actions continue to clear trajectory while retaining the session, so later release cannot create an additional gesture from the wheel-only trajectory.  Existing modifier combinations stay loadable and keep their legacy pass-through behavior; new UI capture deliberately creates a modifier-free dedicated key.

## Validation scope

The pure tests cover dedicated-key selection, duplicate dedicated-key rejection, release-pending entry, chatter resume, confirmed one-time completion, duplicate keyup, wrong key, and state clearing.  Hook code keeps live hook calls and `SendInput` out of these tests.  Physical keyboard, mouse-vendor mapping, Caps Lock/IME, and foreground-application behavior remain manual verification items.
