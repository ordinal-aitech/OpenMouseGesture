using System.Collections.ObjectModel;

namespace GestureHotkeyApp.Models;

public class AppProfile
{
    public string Id { get; set; } = Guid.NewGuid().ToString("N");

    public string Name { get; set; } = "新しいプロファイル";

    public bool Enabled { get; set; } = true;

    public ProfileMode Mode { get; set; } = ProfileMode.Include;

    public ObservableCollection<ProfileMatcher> Matchers { get; set; } = [];

    public ObservableCollection<GestureAction> Actions { get; set; } = [];

    public static AppProfile CreateGlobal()
    {
        return new AppProfile
        {
            Name = "グローバル",
            Enabled = true,
            Mode = ProfileMode.Include
        };
    }

    public void EnsureIds()
    {
        Id = string.IsNullOrWhiteSpace(Id) ? Guid.NewGuid().ToString("N") : Id;

        foreach (var action in Actions)
        {
            action.EnsureIds();
        }
    }
}
