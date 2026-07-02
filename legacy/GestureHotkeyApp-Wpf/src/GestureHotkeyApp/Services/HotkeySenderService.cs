using System.Runtime.InteropServices;
using GestureHotkeyApp.Native;

namespace GestureHotkeyApp.Services;

public class HotkeySenderService
{
    private static readonly Dictionary<string, ushort> SpecialKeys = new(StringComparer.OrdinalIgnoreCase)
    {
        ["LEFT"] = 0x25,
        ["UP"] = 0x26,
        ["RIGHT"] = 0x27,
        ["DOWN"] = 0x28,
        ["ENTER"] = 0x0D,
        ["ESC"] = 0x1B,
        ["ESCAPE"] = 0x1B,
        ["TAB"] = 0x09,
        ["SPACE"] = 0x20,
        ["BACKSPACE"] = 0x08,
        ["DELETE"] = 0x2E,
        ["HOME"] = 0x24,
        ["END"] = 0x23,
        ["PAGEUP"] = 0x21,
        ["PAGEDOWN"] = 0x22,
        ["INSERT"] = 0x2D
    };

    public bool TrySend(string hotkeyText, out string error)
    {
        error = string.Empty;
        if (!TryParse(hotkeyText, out var modifiers, out var keyCode, out error))
        {
            return false;
        }

        var inputs = new List<NativeMethods.INPUT>();
        foreach (var modifier in modifiers)
        {
            inputs.Add(CreateKeyboardInput(modifier, false));
        }

        inputs.Add(CreateKeyboardInput(keyCode, false));
        inputs.Add(CreateKeyboardInput(keyCode, true));

        for (var i = modifiers.Count - 1; i >= 0; i--)
        {
            inputs.Add(CreateKeyboardInput(modifiers[i], true));
        }

        var inputArray = inputs.ToArray();
        var sent = NativeMethods.SendInput((uint)inputArray.Length, inputArray, Marshal.SizeOf<NativeMethods.INPUT>());
        if (sent != inputArray.Length)
        {
            error = "SendInput が失敗しました。";
            return false;
        }

        return true;
    }

    private static bool TryParse(string hotkeyText, out List<ushort> modifiers, out ushort keyCode, out string error)
    {
        modifiers = [];
        keyCode = 0;
        error = string.Empty;

        var tokens = (hotkeyText ?? string.Empty)
            .Split('+', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);

        if (tokens.Length == 0)
        {
            error = "Hotkey が空です。";
            return false;
        }

        foreach (var token in tokens)
        {
            switch (token.ToUpperInvariant())
            {
                case "CTRL":
                case "CONTROL":
                    modifiers.Add(0x11);
                    continue;
                case "ALT":
                    modifiers.Add(0x12);
                    continue;
                case "SHIFT":
                    modifiers.Add(0x10);
                    continue;
                case "WIN":
                case "WINDOWS":
                    modifiers.Add(0x5B);
                    continue;
            }

            if (keyCode != 0)
            {
                error = "通常キーは 1 つだけ指定してください。";
                return false;
            }

            if (!TryResolveKey(token, out keyCode))
            {
                error = $"未対応のキーです: {token}";
                return false;
            }
        }

        if (keyCode == 0)
        {
            error = "通常キーがありません。";
            return false;
        }

        return true;
    }

    private static bool TryResolveKey(string token, out ushort keyCode)
    {
        token = token.Trim();

        if (token.Length == 1)
        {
            var ch = char.ToUpperInvariant(token[0]);
            if (ch is >= 'A' and <= 'Z')
            {
                keyCode = ch;
                return true;
            }

            if (ch is >= '0' and <= '9')
            {
                keyCode = ch;
                return true;
            }
        }

        if (SpecialKeys.TryGetValue(token, out keyCode))
        {
            return true;
        }

        if (token.StartsWith("F", StringComparison.OrdinalIgnoreCase)
            && int.TryParse(token[1..], out var functionKey)
            && functionKey is >= 1 and <= 24)
        {
            keyCode = (ushort)(0x6F + functionKey);
            return true;
        }

        keyCode = 0;
        return false;
    }

    private static NativeMethods.INPUT CreateKeyboardInput(ushort vk, bool keyUp)
    {
        return new NativeMethods.INPUT
        {
            type = NativeMethods.INPUT_KEYBOARD,
            U = new NativeMethods.InputUnion
            {
                ki = new NativeMethods.KEYBDINPUT
                {
                    wVk = vk,
                    dwFlags = keyUp ? NativeMethods.KEYEVENTF_KEYUP : 0
                }
            }
        };
    }
}
