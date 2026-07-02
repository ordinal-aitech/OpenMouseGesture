using System.Text.RegularExpressions;
using GestureHotkeyApp.Models;

namespace GestureHotkeyApp.Services;

public class ProfileResolverService
{
    public bool IsExcluded(AppConfiguration configuration, WindowContext windowContext)
    {
        return configuration.AppProfiles
            .Where(profile => profile.Enabled && profile.Mode == ProfileMode.Exclude)
            .Any(profile => MatchesProfile(profile, windowContext));
    }

    public AppProfile? FindBestIncludeProfile(AppConfiguration configuration, WindowContext windowContext)
    {
        return configuration.AppProfiles
            .Where(profile => profile.Enabled && profile.Mode == ProfileMode.Include)
            .OrderByDescending(profile => profile.Matchers.Count)
            .FirstOrDefault(profile => MatchesProfile(profile, windowContext));
    }

    private static bool MatchesProfile(AppProfile profile, WindowContext context)
    {
        if (profile.Matchers.Count == 0)
        {
            return false;
        }

        return profile.Matchers.Any(matcher => MatchesMatcher(matcher, context));
    }

    private static bool MatchesMatcher(ProfileMatcher matcher, WindowContext context)
    {
        var candidate = matcher.Type switch
        {
            MatcherType.Process => context.ProcessName,
            MatcherType.Class => context.ClassName,
            MatcherType.Title => context.Title,
            _ => string.Empty
        };

        if (matcher.Pattern)
        {
            var regexPattern = "^" + Regex.Escape(matcher.Value)
                .Replace("\\*", ".*", StringComparison.Ordinal)
                .Replace("\\?", ".", StringComparison.Ordinal) + "$";
            return Regex.IsMatch(candidate, regexPattern, RegexOptions.IgnoreCase);
        }

        return string.Equals(candidate, matcher.Value, StringComparison.OrdinalIgnoreCase);
    }
}
