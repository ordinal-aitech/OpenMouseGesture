using System.Runtime.InteropServices;
using GestureHotkeyApp.Models;
using GestureHotkeyApp.Native;
using WpfPoint = System.Windows.Point;

namespace GestureHotkeyApp.Services;

public sealed class MouseHookService : IDisposable
{
    private const double GestureActivationDistance = 10.0;

    private readonly NativeMethods.LowLevelMouseProc _hookCallback;
    private IntPtr _hookHandle = IntPtr.Zero;
    private ActiveGesture? _activeGesture;
    private TriggerButton _triggerAButton = TriggerButton.Right;
    private TriggerButton _triggerBButton = TriggerButton.Middle;
    private long _nextSessionId;

    public MouseHookService()
    {
        _hookCallback = HookCallback;
    }

    public event EventHandler<GestureCapturedEventArgs>? GestureCaptured;

    public event EventHandler<GestureTrailEventArgs>? GestureTrailChanged;

    public void UpdateTriggerButtons(TriggerButton triggerAButton, TriggerButton triggerBButton)
    {
        _triggerAButton = triggerAButton;
        _triggerBButton = triggerBButton;
    }

    public void Start()
    {
        if (_hookHandle != IntPtr.Zero)
        {
            return;
        }

        _hookHandle = NativeMethods.SetWindowsHookEx(
            NativeMethods.WH_MOUSE_LL,
            _hookCallback,
            NativeMethods.GetModuleHandle(IntPtr.Zero),
            0);

        if (_hookHandle == IntPtr.Zero)
        {
            throw new InvalidOperationException("マウスフックの設定に失敗しました。");
        }
    }

    private IntPtr HookCallback(int nCode, IntPtr wParam, IntPtr lParam)
    {
        if (nCode < 0)
        {
            return NativeMethods.CallNextHookEx(_hookHandle, nCode, wParam, lParam);
        }

        var hookStruct = Marshal.PtrToStructure<NativeMethods.MSLLHOOKSTRUCT>(lParam);
        if ((hookStruct.flags & NativeMethods.LLMHF_INJECTED) != 0
            && hookStruct.dwExtraInfo == NativeMethods.GestureHotkeyMouseInputMarker)
        {
            return NativeMethods.CallNextHookEx(_hookHandle, nCode, wParam, lParam);
        }

        var message = (int)wParam;
        var mousePoint = new WpfPoint(hookStruct.pt.x, hookStruct.pt.y);

        if (_activeGesture is null)
        {
            if (TryCreateActiveGesture(message, hookStruct, mousePoint, out var activeGesture))
            {
                _activeGesture = activeGesture;
                return (IntPtr)1;
            }

            return NativeMethods.CallNextHookEx(_hookHandle, nCode, wParam, lParam);
        }

        if (message == NativeMethods.WM_MOUSEMOVE)
        {
            HandleMouseMove(_activeGesture, mousePoint);
            return NativeMethods.CallNextHookEx(_hookHandle, nCode, wParam, lParam);
        }

        if (IsMatchingButtonUp(message, hookStruct, _activeGesture.Button))
        {
            AddPointIfMoved(_activeGesture.Points, mousePoint);
            var completedGesture = _activeGesture;
            _activeGesture = null;

            if (!completedGesture.IsActivated || IsClickLike(completedGesture.Points))
            {
                ReplayOriginalClick(completedGesture.Button);
                PublishTrailChanged(completedGesture, GestureTrailUpdateKind.Clear);
                return (IntPtr)1;
            }

            PublishTrailChanged(completedGesture, GestureTrailUpdateKind.Complete);
            GestureCaptured?.Invoke(this, new GestureCapturedEventArgs(completedGesture.TriggerSlot, completedGesture.Points));
            return (IntPtr)1;
        }

        return NativeMethods.CallNextHookEx(_hookHandle, nCode, wParam, lParam);
    }

    private bool TryCreateActiveGesture(int message, NativeMethods.MSLLHOOKSTRUCT hookStruct, WpfPoint point, out ActiveGesture? gesture)
    {
        gesture = null;
        if (IsButtonDown(message, hookStruct, _triggerAButton))
        {
            gesture = new ActiveGesture(NextSessionId(), TriggerSlot.A, _triggerAButton, [point]);
            return true;
        }

        if (IsButtonDown(message, hookStruct, _triggerBButton))
        {
            gesture = new ActiveGesture(NextSessionId(), TriggerSlot.B, _triggerBButton, [point]);
            return true;
        }

        return false;
    }

    private static bool IsButtonDown(int message, NativeMethods.MSLLHOOKSTRUCT hookStruct, TriggerButton button)
    {
        return button switch
        {
            TriggerButton.Middle => message == NativeMethods.WM_MBUTTONDOWN,
            TriggerButton.Right => message == NativeMethods.WM_RBUTTONDOWN,
            TriggerButton.XButton1 => message == NativeMethods.WM_XBUTTONDOWN && GetXButton(hookStruct.mouseData) == 1,
            TriggerButton.XButton2 => message == NativeMethods.WM_XBUTTONDOWN && GetXButton(hookStruct.mouseData) == 2,
            _ => false
        };
    }

    private static bool IsMatchingButtonUp(int message, NativeMethods.MSLLHOOKSTRUCT hookStruct, TriggerButton button)
    {
        return button switch
        {
            TriggerButton.Middle => message == NativeMethods.WM_MBUTTONUP,
            TriggerButton.Right => message == NativeMethods.WM_RBUTTONUP,
            TriggerButton.XButton1 => message == NativeMethods.WM_XBUTTONUP && GetXButton(hookStruct.mouseData) == 1,
            TriggerButton.XButton2 => message == NativeMethods.WM_XBUTTONUP && GetXButton(hookStruct.mouseData) == 2,
            _ => false
        };
    }

    private static int GetXButton(uint mouseData)
    {
        return (int)((mouseData >> 16) & 0xFFFF);
    }

    private static bool IsClickLike(IReadOnlyList<WpfPoint> points)
    {
        if (points.Count < 2)
        {
            return true;
        }

        var first = points[0];
        var last = points[^1];
        var dx = last.X - first.X;
        var dy = last.Y - first.Y;
        return Math.Sqrt(dx * dx + dy * dy) < 18.0;
    }

    private void HandleMouseMove(ActiveGesture gesture, WpfPoint point)
    {
        AddPointIfMoved(gesture.Points, point);
        if (gesture.IsActivated)
        {
            PublishTrailChanged(gesture, GestureTrailUpdateKind.Update);
            return;
        }

        if (!HasExceededActivationDistance(gesture.Points))
        {
            return;
        }

        gesture.IsActivated = true;
        PublishTrailChanged(gesture, GestureTrailUpdateKind.Start);
    }

    private static bool HasExceededActivationDistance(IReadOnlyList<WpfPoint> points)
    {
        if (points.Count < 2)
        {
            return false;
        }

        var first = points[0];
        var last = points[^1];
        var dx = last.X - first.X;
        var dy = last.Y - first.Y;
        return Math.Sqrt(dx * dx + dy * dy) >= GestureActivationDistance;
    }

    private static void ReplayOriginalClick(TriggerButton button)
    {
        NativeMethods.INPUT[] inputs = button switch
        {
            TriggerButton.Middle =>
            [
                CreateMouseInput(NativeMethods.MOUSEEVENTF_MIDDLEDOWN, 0),
                CreateMouseInput(NativeMethods.MOUSEEVENTF_MIDDLEUP, 0)
            ],
            TriggerButton.Right =>
            [
                CreateMouseInput(NativeMethods.MOUSEEVENTF_RIGHTDOWN, 0),
                CreateMouseInput(NativeMethods.MOUSEEVENTF_RIGHTUP, 0)
            ],
            TriggerButton.XButton1 =>
            [
                CreateMouseInput(NativeMethods.MOUSEEVENTF_XDOWN, 1),
                CreateMouseInput(NativeMethods.MOUSEEVENTF_XUP, 1)
            ],
            TriggerButton.XButton2 =>
            [
                CreateMouseInput(NativeMethods.MOUSEEVENTF_XDOWN, 2),
                CreateMouseInput(NativeMethods.MOUSEEVENTF_XUP, 2)
            ],
            _ => []
        };

        if (inputs.Length > 0)
        {
            NativeMethods.SendInput((uint)inputs.Length, inputs, Marshal.SizeOf<NativeMethods.INPUT>());
        }
    }

    private static NativeMethods.INPUT CreateMouseInput(uint flags, uint mouseData)
    {
        return new NativeMethods.INPUT
        {
            type = NativeMethods.INPUT_MOUSE,
            U = new NativeMethods.InputUnion
            {
                mi = new NativeMethods.MOUSEINPUT
                {
                    dwFlags = flags,
                    mouseData = mouseData,
                    dwExtraInfo = NativeMethods.GestureHotkeyMouseInputMarker
                }
            }
        };
    }

    private static void AddPointIfMoved(List<WpfPoint> points, WpfPoint point)
    {
        if (points.Count == 0 || points[^1] != point)
        {
            points.Add(point);
        }
    }

    private long NextSessionId()
    {
        _nextSessionId++;
        return _nextSessionId;
    }

    private void PublishTrailChanged(ActiveGesture gesture, GestureTrailUpdateKind updateKind)
    {
        GestureTrailChanged?.Invoke(
            this,
            new GestureTrailEventArgs(
                gesture.SessionId,
                gesture.TriggerSlot,
                gesture.Points.ToArray(),
                updateKind));
    }

    public void Dispose()
    {
        if (_hookHandle != IntPtr.Zero)
        {
            NativeMethods.UnhookWindowsHookEx(_hookHandle);
            _hookHandle = IntPtr.Zero;
        }
    }

    private sealed class ActiveGesture
    {
        public ActiveGesture(long sessionId, TriggerSlot triggerSlot, TriggerButton button, List<WpfPoint> points)
        {
            SessionId = sessionId;
            TriggerSlot = triggerSlot;
            Button = button;
            Points = points;
        }

        public long SessionId { get; }

        public TriggerSlot TriggerSlot { get; }

        public TriggerButton Button { get; }

        public List<WpfPoint> Points { get; }

        public bool IsActivated { get; set; }
    }
}

public sealed class GestureCapturedEventArgs : EventArgs
{
    public GestureCapturedEventArgs(TriggerSlot triggerSlot, IReadOnlyList<WpfPoint> points)
    {
        TriggerSlot = triggerSlot;
        Points = points;
    }

    public TriggerSlot TriggerSlot { get; }

    public IReadOnlyList<WpfPoint> Points { get; }
}

public sealed class GestureTrailEventArgs : EventArgs
{
    public GestureTrailEventArgs(long sessionId, TriggerSlot triggerSlot, IReadOnlyList<WpfPoint> points, GestureTrailUpdateKind updateKind)
    {
        SessionId = sessionId;
        TriggerSlot = triggerSlot;
        Points = points;
        UpdateKind = updateKind;
    }

    public long SessionId { get; }

    public TriggerSlot TriggerSlot { get; }

    public IReadOnlyList<WpfPoint> Points { get; }

    public GestureTrailUpdateKind UpdateKind { get; }
}

public enum GestureTrailUpdateKind
{
    Start,
    Update,
    Complete,
    Clear
}
