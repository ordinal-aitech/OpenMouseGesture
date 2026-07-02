using System.Drawing;
using Forms = System.Windows.Forms;

namespace GestureHotkeyApp.Services;

public sealed class TrayIconService : IDisposable
{
    private readonly Forms.NotifyIcon _notifyIcon;
    private readonly Forms.ToolStripMenuItem _toggleItem;

    public TrayIconService()
    {
        _toggleItem = new Forms.ToolStripMenuItem("無効にする");
        var openItem = new Forms.ToolStripMenuItem("設定を開く");
        var exitItem = new Forms.ToolStripMenuItem("終了");

        openItem.Click += (_, _) => OpenRequested?.Invoke(this, EventArgs.Empty);
        _toggleItem.Click += (_, _) => ToggleEnabledRequested?.Invoke(this, EventArgs.Empty);
        exitItem.Click += (_, _) => ExitRequested?.Invoke(this, EventArgs.Empty);

        var menu = new Forms.ContextMenuStrip();
        menu.Items.Add(openItem);
        menu.Items.Add(_toggleItem);
        menu.Items.Add(new Forms.ToolStripSeparator());
        menu.Items.Add(exitItem);

        _notifyIcon = new Forms.NotifyIcon
        {
            Icon = SystemIcons.Application,
            Visible = true,
            Text = "ジェスチャーHotkey",
            ContextMenuStrip = menu
        };

        _notifyIcon.DoubleClick += (_, _) => OpenRequested?.Invoke(this, EventArgs.Empty);
    }

    public event EventHandler? OpenRequested;

    public event EventHandler? ToggleEnabledRequested;

    public event EventHandler? ExitRequested;

    public void UpdateState(bool enabled)
    {
        _toggleItem.Text = enabled ? "無効にする" : "有効にする";
        _notifyIcon.Text = enabled ? "ジェスチャーHotkey (有効)" : "ジェスチャーHotkey (無効)";
    }

    public void Dispose()
    {
        _notifyIcon.Visible = false;
        _notifyIcon.Dispose();
    }
}
