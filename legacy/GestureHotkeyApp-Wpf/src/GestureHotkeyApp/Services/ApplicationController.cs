using GestureHotkeyApp.Models;

namespace GestureHotkeyApp.Services;

public sealed class ApplicationController : IDisposable
{
    private readonly JsonConfigurationService _configurationService;
    private readonly StartupRegistrationService _startupRegistrationService;
    private readonly GestureRecognitionService _gestureRecognitionService;
    private readonly ProfileResolverService _profileResolverService;
    private readonly WindowInfoService _windowInfoService;
    private readonly HotkeySenderService _hotkeySenderService;
    private readonly MouseHookService _mouseHookService;
    private string? _startupRegistrationWarning;

    public ApplicationController(
        JsonConfigurationService configurationService,
        StartupRegistrationService startupRegistrationService,
        GestureRecognitionService gestureRecognitionService,
        ProfileResolverService profileResolverService,
        WindowInfoService windowInfoService,
        HotkeySenderService hotkeySenderService,
        MouseHookService mouseHookService)
    {
        _configurationService = configurationService;
        _startupRegistrationService = startupRegistrationService;
        _gestureRecognitionService = gestureRecognitionService;
        _profileResolverService = profileResolverService;
        _windowInfoService = windowInfoService;
        _hotkeySenderService = hotkeySenderService;
        _mouseHookService = mouseHookService;
    }

    public event EventHandler? ConfigurationChanged;

    public event EventHandler<string>? StatusMessageUpdated;

    public event EventHandler<string>? LastGestureUpdated;

    public AppConfiguration Configuration { get; private set; } = AppConfiguration.CreateDefault();

    public bool ExitRequested { get; private set; }

    public string ConfigPath => _configurationService.ConfigPath;

    public void Initialize()
    {
        Configuration = _configurationService.LoadOrCreate();
        ApplyRuntimeSettings();
        _configurationService.Save(Configuration);
        _mouseHookService.GestureCaptured += MouseHookService_GestureCaptured;
        _mouseHookService.Start();
        PublishStatus(_startupRegistrationWarning ?? "常駐を開始しました。");
        OnConfigurationChanged();
    }

    public void SaveConfiguration(AppConfiguration configuration)
    {
        Configuration = configuration;
        Configuration.Normalize();
        _configurationService.Save(Configuration);
        ApplyRuntimeSettings();
        PublishStatus("設定を保存しました。");
        OnConfigurationChanged();
    }

    public AppConfiguration ReloadConfiguration()
    {
        Configuration = _configurationService.LoadOrCreate();
        ApplyRuntimeSettings();
        _configurationService.Save(Configuration);
        PublishStatus("設定を再読み込みしました。");
        OnConfigurationChanged();
        return Configuration;
    }

    public AppConfiguration ImportConfiguration(string path)
    {
        Configuration = _configurationService.Import(path);
        Configuration.Normalize();
        _configurationService.Save(Configuration);
        ApplyRuntimeSettings();
        PublishStatus("設定をインポートしました。");
        OnConfigurationChanged();
        return Configuration;
    }

    public void ExportConfiguration(string path)
    {
        _configurationService.Export(Configuration, path);
        PublishStatus("設定をエクスポートしました。");
    }

    public void SetEnabled(bool enabled)
    {
        if (Configuration.IsEnabled == enabled)
        {
            return;
        }

        Configuration.IsEnabled = enabled;
        PersistRuntimeSettings();
        PublishStatus(enabled ? "アプリを有効にしました。" : "アプリを無効にしました。");
        OnConfigurationChanged();
    }

    public void SetStartWithWindows(bool enabled)
    {
        if (Configuration.StartWithWindows == enabled)
        {
            return;
        }

        Configuration.StartWithWindows = enabled;
        PersistRuntimeSettings();
        PublishStatus(_startupRegistrationWarning
            ?? (enabled ? "Windows 起動時に開始する設定にしました。" : "Windows 起動時に開始しない設定にしました。"));
        OnConfigurationChanged();
    }

    public bool TryUpdateTriggerButtons(TriggerButton triggerAButton, TriggerButton triggerBButton)
    {
        if (triggerAButton == triggerBButton)
        {
            PublishStatus("Trigger A と Trigger B に同じ開始ボタンは設定できません。");
            return false;
        }

        Configuration.TriggerSettings.TriggerAButton = triggerAButton;
        Configuration.TriggerSettings.TriggerBButton = triggerBButton;
        PersistRuntimeSettings();
        ApplyRuntimeSettings();
        PublishStatus("開始ボタン設定を更新しました。");
        OnConfigurationChanged();
        return true;
    }

    public void RequestExit()
    {
        ExitRequested = true;
    }

    private void PersistRuntimeSettings()
    {
        Configuration.Normalize();
        _configurationService.Save(Configuration);
        ApplyStartupRegistration();
    }

    private void ApplyRuntimeSettings()
    {
        Configuration.Normalize();
        ApplyStartupRegistration();
        _mouseHookService.UpdateTriggerButtons(
            Configuration.TriggerSettings.TriggerAButton,
            Configuration.TriggerSettings.TriggerBButton);
    }

    private void ApplyStartupRegistration()
    {
        var result = _startupRegistrationService.SetEnabled(Configuration.StartWithWindows);
        _startupRegistrationWarning = result.Success
            ? null
            : $"Windows 起動時の自動起動設定に失敗しました: {result.ErrorMessage}";
    }

    private void MouseHookService_GestureCaptured(object? sender, GestureCapturedEventArgs e)
    {
        var pattern = _gestureRecognitionService.Recognize(e.Points);
        if (pattern is null)
        {
            PublishStatus("ジェスチャーが短すぎるため無視しました。");
            return;
        }

        LastGestureUpdated?.Invoke(this, $"Trigger {e.TriggerSlot}: {pattern}");

        if (!Configuration.IsEnabled)
        {
            PublishStatus("アプリが無効のため Hotkey は送信しません。");
            return;
        }

        var windowContext = _windowInfoService.GetForegroundWindowContext();
        if (_profileResolverService.IsExcluded(Configuration, windowContext))
        {
            PublishStatus("除外プロファイルに一致したため実行しません。");
            return;
        }

        var matchedProfile = _profileResolverService.FindBestIncludeProfile(Configuration, windowContext);
        var action = FindAction(matchedProfile, e.TriggerSlot, pattern)
                     ?? FindAction(Configuration.GlobalProfile, e.TriggerSlot, pattern);

        if (action is null)
        {
            PublishStatus($"ジェスチャー '{pattern}' に対応する Hotkey がありません。");
            return;
        }

        if (_hotkeySenderService.TrySend(action.Hotkey.Text, out var error))
        {
            var profileName = matchedProfile?.Name ?? "グローバル";
            PublishStatus($"{profileName} / {action.Name} を実行しました。");
        }
        else
        {
            PublishStatus($"Hotkey 送信に失敗しました: {error}");
        }
    }

    private static GestureAction? FindAction(AppProfile? profile, TriggerSlot slot, string pattern)
    {
        if (profile is null || !profile.Enabled)
        {
            return null;
        }

        return profile.Actions.FirstOrDefault(action =>
            action.TriggerSlot == slot &&
            action.GesturePatterns.Any(gesture =>
                string.Equals(gesture, pattern, StringComparison.OrdinalIgnoreCase)));
    }

    private void PublishStatus(string message)
    {
        StatusMessageUpdated?.Invoke(this, message);
    }

    private void OnConfigurationChanged()
    {
        ConfigurationChanged?.Invoke(this, EventArgs.Empty);
    }

    public void Dispose()
    {
        _mouseHookService.GestureCaptured -= MouseHookService_GestureCaptured;
        _mouseHookService.Dispose();
    }
}
