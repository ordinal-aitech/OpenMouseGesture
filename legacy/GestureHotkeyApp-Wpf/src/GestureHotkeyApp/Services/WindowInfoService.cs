using System.Diagnostics;
using System.Text;
using GestureHotkeyApp.Models;
using GestureHotkeyApp.Native;

namespace GestureHotkeyApp.Services;

public class WindowInfoService
{
    public WindowContext GetForegroundWindowContext()
    {
        var handle = NativeMethods.GetForegroundWindow();
        if (handle == IntPtr.Zero)
        {
            return new WindowContext();
        }

        NativeMethods.GetWindowThreadProcessId(handle, out var processId);
        var processName = string.Empty;
        try
        {
            processName = Process.GetProcessById((int)processId).ProcessName + ".exe";
        }
        catch
        {
            processName = string.Empty;
        }

        var classBuilder = new StringBuilder(256);
        NativeMethods.GetClassName(handle, classBuilder, classBuilder.Capacity);

        var titleBuilder = new StringBuilder(512);
        NativeMethods.GetWindowText(handle, titleBuilder, titleBuilder.Capacity);

        return new WindowContext
        {
            ProcessName = processName,
            ClassName = classBuilder.ToString(),
            Title = titleBuilder.ToString()
        };
    }
}
