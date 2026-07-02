using GestureHotkeyApp.Services;
using GestureHotkeyApp.ViewModels;
using Wpf = System.Windows;

namespace GestureHotkeyApp;

public partial class App : System.Windows.Application
{
    private ApplicationController? _controller;
    private GestureTrailOverlayService? _gestureTrailOverlayService;
    private TrayIconService? _trayIconService;
    private MainWindow? _mainWindow;

    protected override void OnStartup(Wpf.StartupEventArgs e)
    {
        base.OnStartup(e);

        var configurationService = new JsonConfigurationService();
        var startupService = new StartupRegistrationService();
        var gestureRecognitionService = new GestureRecognitionService();
        var profileResolverService = new ProfileResolverService();
        var windowInfoService = new WindowInfoService();
        var hotkeySenderService = new HotkeySenderService();
        var mouseHookService = new MouseHookService();
        _gestureTrailOverlayService = new GestureTrailOverlayService();
        mouseHookService.GestureTrailChanged += MouseHookService_GestureTrailChanged;

        _controller = new ApplicationController(
            configurationService,
            startupService,
            gestureRecognitionService,
            profileResolverService,
            windowInfoService,
            hotkeySenderService,
            mouseHookService);

        _controller.Initialize();
        _gestureTrailOverlayService.ApplySettings(_controller.Configuration.UiSettings);

        var viewModel = new MainWindowViewModel(_controller);
        _mainWindow = new MainWindow(viewModel, _controller);
        MainWindow = _mainWindow;
        _mainWindow.Show();

        _trayIconService = new TrayIconService();
        _trayIconService.OpenRequested += (_, _) => ShowMainWindow();
        _trayIconService.ToggleEnabledRequested += (_, _) =>
        {
            if (_controller is not null)
            {
                _controller.SetEnabled(!_controller.Configuration.IsEnabled);
                _trayIconService.UpdateState(_controller.Configuration.IsEnabled);
            }
        };
        _trayIconService.ExitRequested += (_, _) => ExitApplication();
        _trayIconService.UpdateState(_controller.Configuration.IsEnabled);

        _controller.ConfigurationChanged += (_, _) =>
        {
            if (_controller is not null)
            {
                _trayIconService.UpdateState(_controller.Configuration.IsEnabled);
                _gestureTrailOverlayService?.ApplySettings(_controller.Configuration.UiSettings);
            }
        };
    }

    protected override void OnExit(Wpf.ExitEventArgs e)
    {
        _trayIconService?.Dispose();
        _gestureTrailOverlayService?.Dispose();
        _controller?.Dispose();
        base.OnExit(e);
    }

    private void MouseHookService_GestureTrailChanged(object? sender, GestureTrailEventArgs e)
    {
        _gestureTrailOverlayService?.ShowTrail(e.SessionId, e.TriggerSlot, e.Points, e.UpdateKind);
    }

    private void ShowMainWindow()
    {
        if (_mainWindow is null)
        {
            return;
        }

        if (!_mainWindow.IsVisible)
        {
            _mainWindow.Show();
        }

        if (_mainWindow.WindowState == Wpf.WindowState.Minimized)
        {
            _mainWindow.WindowState = Wpf.WindowState.Normal;
        }

        _mainWindow.Activate();
        _mainWindow.Topmost = true;
        _mainWindow.Topmost = false;
        _mainWindow.Focus();
    }

    private void ExitApplication()
    {
        if (_controller is not null)
        {
            _controller.RequestExit();
        }

        _mainWindow?.ForceClose();
        Shutdown();
    }
}
