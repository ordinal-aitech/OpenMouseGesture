using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using GestureHotkeyApp.Models;
using GestureHotkeyApp.Services;

namespace GestureHotkeyApp.ViewModels;

public class MainWindowViewModel : INotifyPropertyChanged
{
    private readonly ApplicationController _controller;
    private AppConfiguration _configuration;
    private AppProfile? _selectedProfile;
    private ProfileMatcher? _selectedMatcher;
    private GestureAction? _selectedProfileAction;
    private GestureAction? _selectedGlobalAction;
    private string _statusMessage = "準備完了";
    private string _lastGestureSummary = "まだ実行されていません。";

    public MainWindowViewModel(ApplicationController controller)
    {
        _controller = controller;
        _configuration = controller.Configuration;

        _controller.ConfigurationChanged += (_, _) =>
        {
            _configuration = _controller.Configuration;
            if (_selectedProfile is not null && !_configuration.AppProfiles.Contains(_selectedProfile))
            {
                _selectedProfile = _configuration.AppProfiles.FirstOrDefault();
            }

            OnPropertyChanged(string.Empty);
        };
        _controller.StatusMessageUpdated += (_, message) => StatusMessage = message;
        _controller.LastGestureUpdated += (_, message) => LastGestureSummary = message;
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public IReadOnlyList<TriggerButton> TriggerButtonOptions { get; } = Enum.GetValues<TriggerButton>();

    public IReadOnlyList<ProfileMode> ProfileModeOptions { get; } = Enum.GetValues<ProfileMode>();

    public AppConfiguration Configuration => _configuration;

    public ObservableCollection<GestureAction> GlobalActions => _configuration.GlobalProfile.Actions;

    public ObservableCollection<AppProfile> AppProfiles => _configuration.AppProfiles;

    public string ConfigPath => _controller.ConfigPath;

    public bool IsEnabled
    {
        get => _configuration.IsEnabled;
        set
        {
            if (_configuration.IsEnabled == value)
            {
                return;
            }

            _controller.SetEnabled(value);
            OnPropertyChanged();
        }
    }

    public bool StartWithWindows
    {
        get => _configuration.StartWithWindows;
        set
        {
            if (_configuration.StartWithWindows == value)
            {
                return;
            }

            _controller.SetStartWithWindows(value);
            OnPropertyChanged();
        }
    }

    public TriggerButton TriggerAButton
    {
        get => _configuration.TriggerSettings.TriggerAButton;
        set
        {
            if (_configuration.TriggerSettings.TriggerAButton == value)
            {
                return;
            }

            var targetTriggerA = value;
            var targetTriggerB = value == TriggerBButton
                ? _configuration.TriggerSettings.TriggerAButton
                : TriggerBButton;

            if (_controller.TryUpdateTriggerButtons(targetTriggerA, targetTriggerB))
            {
                OnPropertyChanged();
                OnPropertyChanged(nameof(TriggerBButton));
            }
            else
            {
                OnPropertyChanged();
            }
        }
    }

    public TriggerButton TriggerBButton
    {
        get => _configuration.TriggerSettings.TriggerBButton;
        set
        {
            if (_configuration.TriggerSettings.TriggerBButton == value)
            {
                return;
            }

            var targetTriggerA = value == TriggerAButton
                ? _configuration.TriggerSettings.TriggerBButton
                : TriggerAButton;
            var targetTriggerB = value;

            if (_controller.TryUpdateTriggerButtons(targetTriggerA, targetTriggerB))
            {
                OnPropertyChanged();
                OnPropertyChanged(nameof(TriggerAButton));
            }
            else
            {
                OnPropertyChanged();
            }
        }
    }

    public AppProfile? SelectedProfile
    {
        get => _selectedProfile;
        set
        {
            if (_selectedProfile == value)
            {
                return;
            }

            _selectedProfile = value;
            SelectedMatcher = null;
            SelectedProfileAction = null;
            OnPropertyChanged();
        }
    }

    public ProfileMatcher? SelectedMatcher
    {
        get => _selectedMatcher;
        set
        {
            _selectedMatcher = value;
            OnPropertyChanged();
        }
    }

    public GestureAction? SelectedProfileAction
    {
        get => _selectedProfileAction;
        set
        {
            _selectedProfileAction = value;
            OnPropertyChanged();
        }
    }

    public GestureAction? SelectedGlobalAction
    {
        get => _selectedGlobalAction;
        set
        {
            _selectedGlobalAction = value;
            OnPropertyChanged();
        }
    }

    public string StatusMessage
    {
        get => _statusMessage;
        private set
        {
            _statusMessage = value;
            OnPropertyChanged();
        }
    }

    public string LastGestureSummary
    {
        get => _lastGestureSummary;
        private set
        {
            _lastGestureSummary = value;
            OnPropertyChanged();
        }
    }

    public void SaveConfiguration()
    {
        _controller.SaveConfiguration(_configuration);
    }

    public void ReloadConfiguration()
    {
        _controller.ReloadConfiguration();
    }

    public void ExportConfiguration(string path)
    {
        _controller.ExportConfiguration(path);
    }

    public void ImportConfiguration(string path)
    {
        _controller.ImportConfiguration(path);
    }

    public void AddGlobalAction()
    {
        var action = new GestureAction();
        GlobalActions.Add(action);
        SelectedGlobalAction = action;
    }

    public void RemoveSelectedGlobalAction()
    {
        if (SelectedGlobalAction is null)
        {
            return;
        }

        GlobalActions.Remove(SelectedGlobalAction);
        SelectedGlobalAction = null;
    }

    public void AddProfile()
    {
        var profile = new AppProfile
        {
            Name = $"新しいプロファイル {AppProfiles.Count + 1}",
            Mode = ProfileMode.Include
        };

        AppProfiles.Add(profile);
        SelectedProfile = profile;
    }

    public void RemoveSelectedProfile()
    {
        if (SelectedProfile is null)
        {
            return;
        }

        AppProfiles.Remove(SelectedProfile);
        SelectedProfile = AppProfiles.FirstOrDefault();
    }

    public void AddMatcher()
    {
        if (SelectedProfile is null)
        {
            return;
        }

        var matcher = new ProfileMatcher();
        SelectedProfile.Matchers.Add(matcher);
        SelectedMatcher = matcher;
    }

    public void RemoveSelectedMatcher()
    {
        if (SelectedProfile is null || SelectedMatcher is null)
        {
            return;
        }

        SelectedProfile.Matchers.Remove(SelectedMatcher);
        SelectedMatcher = null;
    }

    public void AddProfileAction()
    {
        if (SelectedProfile is null)
        {
            return;
        }

        var action = new GestureAction();
        SelectedProfile.Actions.Add(action);
        SelectedProfileAction = action;
    }

    public void RemoveSelectedProfileAction()
    {
        if (SelectedProfile is null || SelectedProfileAction is null)
        {
            return;
        }

        SelectedProfile.Actions.Remove(SelectedProfileAction);
        SelectedProfileAction = null;
    }

    private void OnPropertyChanged([CallerMemberName] string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}
