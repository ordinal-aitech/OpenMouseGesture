using System.IO;
using System.Text.Json;
using System.Text.Json.Serialization;
using GestureHotkeyApp.Models;

namespace GestureHotkeyApp.Services;

public class JsonConfigurationService
{
    private readonly JsonSerializerOptions _serializerOptions = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() }
    };

    public string ConfigDirectory { get; } = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "GestureHotkeyApp");

    public string ConfigPath => Path.Combine(ConfigDirectory, "config.json");

    public AppConfiguration LoadOrCreate()
    {
        Directory.CreateDirectory(ConfigDirectory);
        if (!File.Exists(ConfigPath))
        {
            var created = AppConfiguration.CreateDefault();
            Save(created);
            return created;
        }

        var json = File.ReadAllText(ConfigPath);
        var configuration = JsonSerializer.Deserialize<AppConfiguration>(json, _serializerOptions)
                            ?? AppConfiguration.CreateDefault();
        configuration.Normalize();
        return configuration;
    }

    public void Save(AppConfiguration configuration)
    {
        Directory.CreateDirectory(ConfigDirectory);
        configuration.Normalize();
        var json = JsonSerializer.Serialize(configuration, _serializerOptions);
        File.WriteAllText(ConfigPath, json);
    }

    public void Export(AppConfiguration configuration, string path)
    {
        configuration.Normalize();
        var json = JsonSerializer.Serialize(configuration, _serializerOptions);
        File.WriteAllText(path, json);
    }

    public AppConfiguration Import(string path)
    {
        var json = File.ReadAllText(path);
        var configuration = JsonSerializer.Deserialize<AppConfiguration>(json, _serializerOptions)
                            ?? AppConfiguration.CreateDefault();
        configuration.Normalize();
        return configuration;
    }
}
