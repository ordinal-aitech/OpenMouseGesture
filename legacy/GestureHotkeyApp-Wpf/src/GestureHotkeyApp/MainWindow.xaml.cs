using System.ComponentModel;
using System.Windows;
using GestureHotkeyApp.Models;
using GestureHotkeyApp.Services;
using GestureHotkeyApp.ViewModels;

namespace GestureHotkeyApp;

public partial class MainWindow : Window
{
    private readonly ApplicationController _controller;
    private bool _forceClose;

    public MainWindow(MainWindowViewModel viewModel, ApplicationController controller)
    {
        InitializeComponent();
        DataContext = viewModel;
        _controller = controller;

        GlobalTriggerSlotColumn.ItemsSource = Enum.GetValues<TriggerSlot>();
        ProfileTriggerSlotColumn.ItemsSource = Enum.GetValues<TriggerSlot>();
        MatcherTypeColumn.ItemsSource = Enum.GetValues<MatcherType>();
    }

    private MainWindowViewModel ViewModel => (MainWindowViewModel)DataContext;

    public void ForceClose()
    {
        _forceClose = true;
        Close();
    }

    protected override void OnClosing(CancelEventArgs e)
    {
        if (!_forceClose && !_controller.ExitRequested)
        {
            e.Cancel = true;
            Hide();
            return;
        }

        base.OnClosing(e);
    }

    private void SaveButton_Click(object sender, RoutedEventArgs e) => ViewModel.SaveConfiguration();

    private void ReloadButton_Click(object sender, RoutedEventArgs e) => ViewModel.ReloadConfiguration();

    private void ExportButton_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new Microsoft.Win32.SaveFileDialog
        {
            Filter = "JSON ファイル (*.json)|*.json",
            FileName = "gesture-hotkey-config.json",
            Title = "設定をエクスポート"
        };

        if (dialog.ShowDialog(this) == true)
        {
            ViewModel.ExportConfiguration(dialog.FileName);
        }
    }

    private void ImportButton_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new Microsoft.Win32.OpenFileDialog
        {
            Filter = "JSON ファイル (*.json)|*.json",
            Title = "設定をインポート"
        };

        if (dialog.ShowDialog(this) == true)
        {
            ViewModel.ImportConfiguration(dialog.FileName);
        }
    }

    private void AddGlobalActionButton_Click(object sender, RoutedEventArgs e) => ViewModel.AddGlobalAction();

    private void RemoveGlobalActionButton_Click(object sender, RoutedEventArgs e) => ViewModel.RemoveSelectedGlobalAction();

    private void AddProfileButton_Click(object sender, RoutedEventArgs e) => ViewModel.AddProfile();

    private void RemoveProfileButton_Click(object sender, RoutedEventArgs e) => ViewModel.RemoveSelectedProfile();

    private void AddMatcherButton_Click(object sender, RoutedEventArgs e) => ViewModel.AddMatcher();

    private void RemoveMatcherButton_Click(object sender, RoutedEventArgs e) => ViewModel.RemoveSelectedMatcher();

    private void AddProfileActionButton_Click(object sender, RoutedEventArgs e) => ViewModel.AddProfileAction();

    private void RemoveProfileActionButton_Click(object sender, RoutedEventArgs e) => ViewModel.RemoveSelectedProfileAction();
}
