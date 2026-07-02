using System.Collections.ObjectModel;
using System.Text.Json.Serialization;

namespace GestureHotkeyApp.Models;

public class GestureAction
{
    public string Id { get; set; } = Guid.NewGuid().ToString("N");

    public string Name { get; set; } = "新しいアクション";

    public TriggerSlot TriggerSlot { get; set; } = TriggerSlot.A;

    public ObservableCollection<string> GesturePatterns { get; set; } = ["R"];

    public HotkeyDefinition Hotkey { get; set; } = new();

    [JsonIgnore]
    public string GesturePatternsText
    {
        get => string.Join(", ", GesturePatterns);
        set
        {
            var parts = (value ?? string.Empty)
                .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                .Select(static x => x.ToUpperInvariant())
                .Distinct(StringComparer.OrdinalIgnoreCase)
                .ToList();

            GesturePatterns.Clear();
            foreach (var part in parts)
            {
                GesturePatterns.Add(part);
            }
        }
    }

    [JsonIgnore]
    public string HotkeyText
    {
        get => Hotkey.Text;
        set => Hotkey.Text = value?.Trim() ?? string.Empty;
    }

    public void EnsureIds()
    {
        Id = string.IsNullOrWhiteSpace(Id) ? Guid.NewGuid().ToString("N") : Id;
        Hotkey ??= new HotkeyDefinition();
        GesturePatterns ??= [];
    }
}
