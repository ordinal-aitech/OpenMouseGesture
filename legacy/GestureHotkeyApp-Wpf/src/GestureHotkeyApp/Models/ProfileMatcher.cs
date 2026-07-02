namespace GestureHotkeyApp.Models;

public class ProfileMatcher
{
    public MatcherType Type { get; set; } = MatcherType.Process;

    public string Value { get; set; } = string.Empty;

    public bool Pattern { get; set; }
}
