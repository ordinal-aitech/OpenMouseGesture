namespace GestureHotkeyApp.Models;

public class UiSettings
{
    public bool DrawTrail { get; set; } = true;

    public int TrailWidth { get; set; } = 4;

    public int TrailFadeOutMilliseconds { get; set; } = 450;

    public void Normalize()
    {
        if (TrailWidth < 1)
        {
            TrailWidth = 1;
        }

        if (TrailFadeOutMilliseconds < 0)
        {
            TrailFadeOutMilliseconds = 0;
        }
    }
}
