namespace GestureHotkeyApp.Models;

public class TriggerSettings
{
    public TriggerButton TriggerAButton { get; set; } = TriggerButton.Right;

    public TriggerButton TriggerBButton { get; set; } = TriggerButton.Middle;

    public int GestureTimeoutMilliseconds { get; set; } = 1200;
}
