using System.Diagnostics;
using System.IO;
using Microsoft.Win32;

namespace GestureHotkeyApp.Services;

public class StartupRegistrationService
{
    private const string RunKeyPath = @"Software\Microsoft\Windows\CurrentVersion\Run";
    private const string AppName = "GestureHotkeyApp";

    public StartupRegistrationResult SetEnabled(bool enabled)
    {
        try
        {
            using var key = Registry.CurrentUser.CreateSubKey(RunKeyPath);
            if (key is null)
            {
                return StartupRegistrationResult.FailureResult("自動起動レジストリを開けませんでした。");
            }

            if (!enabled)
            {
                key.DeleteValue(AppName, false);
                var registeredCommand = key.GetValue(AppName) as string;
                return registeredCommand is null
                    ? StartupRegistrationResult.DisabledResult()
                    : StartupRegistrationResult.FailureResult("自動起動レジストリの削除を確認できませんでした。");
            }

            var startupCommand = ResolveStartupCommand();
            if (string.IsNullOrWhiteSpace(startupCommand))
            {
                return StartupRegistrationResult.FailureResult("自動起動コマンドを解決できませんでした。");
            }

            key.SetValue(AppName, startupCommand, RegistryValueKind.String);
            var actualCommand = key.GetValue(AppName) as string;
            return string.Equals(actualCommand, startupCommand, StringComparison.Ordinal)
                ? StartupRegistrationResult.EnabledResult(actualCommand)
                : StartupRegistrationResult.FailureResult("自動起動レジストリの書き込み結果を確認できませんでした。");
        }
        catch (Exception ex)
        {
            return StartupRegistrationResult.FailureResult(ex.Message);
        }
    }

    private static string? ResolveStartupCommand()
    {
        var executablePath = Process.GetCurrentProcess().MainModule?.FileName;
        if (!string.IsNullOrWhiteSpace(executablePath))
        {
            return Quote(executablePath);
        }

        executablePath = Environment.ProcessPath;
        if (!string.IsNullOrWhiteSpace(executablePath))
        {
            return Quote(executablePath);
        }

        var entryAssemblyPath = Environment.GetCommandLineArgs().FirstOrDefault();
        if (!string.IsNullOrWhiteSpace(entryAssemblyPath))
        {
            var fullPath = Path.GetFullPath(entryAssemblyPath);
            return Quote(fullPath);
        }

        return null;
    }

    private static string Quote(string path)
    {
        return $"\"{path}\"";
    }
}

public sealed record StartupRegistrationResult(bool Success, bool Enabled, string? Command, string? ErrorMessage)
{
    public static StartupRegistrationResult EnabledResult(string? command) => new(true, true, command, null);

    public static StartupRegistrationResult DisabledResult() => new(true, false, null, null);

    public static StartupRegistrationResult FailureResult(string errorMessage) => new(false, false, null, errorMessage);
}
