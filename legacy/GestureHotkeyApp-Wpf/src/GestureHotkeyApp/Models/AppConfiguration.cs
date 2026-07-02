using System.Collections.ObjectModel;

namespace GestureHotkeyApp.Models;

public class AppConfiguration
{
    public int SchemaVersion { get; set; } = 2;

    public bool IsEnabled { get; set; } = true;

    public bool StartWithWindows { get; set; }

    public TriggerSettings TriggerSettings { get; set; } = new();

    public UiSettings UiSettings { get; set; } = new();

    public AppProfile GlobalProfile { get; set; } = AppProfile.CreateGlobal();

    public ObservableCollection<AppProfile> AppProfiles { get; set; } = [];

    public void Normalize()
    {
        TriggerSettings ??= new TriggerSettings();
        UiSettings ??= new UiSettings();

        if (SchemaVersion < 2)
        {
            UiSettings.DrawTrail = true;
            UiSettings.TrailWidth = UiSettings.TrailWidth <= 0 ? 4 : UiSettings.TrailWidth;
            UiSettings.TrailFadeOutMilliseconds = UiSettings.TrailFadeOutMilliseconds <= 0
                ? 450
                : UiSettings.TrailFadeOutMilliseconds;
            SchemaVersion = 2;
        }

        UiSettings.Normalize();
        GlobalProfile ??= AppProfile.CreateGlobal();
        GlobalProfile.Name = "グローバル";
        GlobalProfile.EnsureIds();

        foreach (var profile in AppProfiles)
        {
            profile.EnsureIds();
        }
    }

    public static AppConfiguration CreateDefault()
    {
        var config = new AppConfiguration
        {
            IsEnabled = true,
            StartWithWindows = false,
            TriggerSettings = new TriggerSettings
            {
                TriggerAButton = TriggerButton.Right,
                TriggerBButton = TriggerButton.Middle
            }
        };

        config.GlobalProfile.Actions.Add(new GestureAction
        {
            Name = "戻る",
            TriggerSlot = TriggerSlot.A,
            GesturePatterns = ["L"],
            Hotkey = new HotkeyDefinition { Text = "Alt+Left" }
        });

        config.GlobalProfile.Actions.Add(new GestureAction
        {
            Name = "進む",
            TriggerSlot = TriggerSlot.A,
            GesturePatterns = ["R"],
            Hotkey = new HotkeyDefinition { Text = "Alt+Right" }
        });

        config.Normalize();
        return config;
    }
}
