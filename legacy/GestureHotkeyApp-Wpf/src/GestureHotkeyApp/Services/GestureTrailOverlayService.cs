using System.Linq;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Shapes;
using System.Windows.Threading;
using GestureHotkeyApp.Models;
using GestureHotkeyApp.Native;
using Forms = System.Windows.Forms;
using MediaBrush = System.Windows.Media.Brush;
using MediaColor = System.Windows.Media.Color;
using WpfPoint = System.Windows.Point;

namespace GestureHotkeyApp.Services;

public sealed class GestureTrailOverlayService : IDisposable
{
    private readonly object _updateSync = new();
    private readonly Dispatcher _dispatcher;
    private readonly DispatcherTimer _clearTimer;
    private readonly GestureOverlayWindow _window;
    private readonly SolidColorBrush _triggerABrush;
    private readonly SolidColorBrush _triggerBBrush;
    private bool _isEnabled = true;
    private int _trailWidth = 4;
    private bool _disposed;
    private bool _isInitialized;
    private bool _renderScheduled;
    private long _activeSessionId;
    private long? _clearSessionId;
    private TrailUpdate? _pendingUpdate;

    public GestureTrailOverlayService()
    {
        _dispatcher = System.Windows.Application.Current?.Dispatcher ?? Dispatcher.CurrentDispatcher;
        _clearTimer = new DispatcherTimer(DispatcherPriority.Render, _dispatcher);
        _clearTimer.Tick += ClearTimer_Tick;
        _window = new GestureOverlayWindow();
        _triggerABrush = CreateFrozenBrush(MediaColor.FromRgb(255, 72, 72));
        _triggerBBrush = CreateFrozenBrush(MediaColor.FromRgb(72, 136, 255));
        RunOnDispatcher(EnsureInitialized);
    }

    public void ApplySettings(UiSettings settings)
    {
        if (settings is null)
        {
            return;
        }

        RunOnDispatcher(() =>
        {
            _isEnabled = settings.DrawTrail;
            _trailWidth = Math.Max(1, settings.TrailWidth);
            _clearTimer.Interval = TimeSpan.FromMilliseconds(Math.Max(0, settings.TrailFadeOutMilliseconds));

            if (!_isEnabled)
            {
                _clearTimer.Stop();
                _window.ClearTrail();
            }
        });
    }

    public void ShowTrail(long sessionId, TriggerSlot triggerSlot, IReadOnlyList<WpfPoint> points, GestureTrailUpdateKind updateKind)
    {
        if (updateKind != GestureTrailUpdateKind.Clear && points.Count == 0)
        {
            return;
        }

        lock (_updateSync)
        {
            if (updateKind == GestureTrailUpdateKind.Start)
            {
                _activeSessionId = sessionId;
                _clearSessionId = null;
                _pendingUpdate = new TrailUpdate(sessionId, triggerSlot, points.ToArray(), updateKind);
            }
            else
            {
                if (sessionId < _activeSessionId)
                {
                    return;
                }

                if (updateKind == GestureTrailUpdateKind.Clear)
                {
                    _clearSessionId = null;
                }

                _pendingUpdate = new TrailUpdate(sessionId, triggerSlot, points.ToArray(), updateKind);
            }

            if (_renderScheduled)
            {
                return;
            }

            _renderScheduled = true;
        }

        QueueRender(updateKind == GestureTrailUpdateKind.Start
            ? DispatcherPriority.Send
            : DispatcherPriority.Render);
    }

    private void EnsureInitialized()
    {
        if (_isInitialized)
        {
            return;
        }

        _window.Prepare();
        _isInitialized = true;
    }

    private MediaBrush GetBrush(TriggerSlot triggerSlot)
    {
        return triggerSlot == TriggerSlot.A ? _triggerABrush : _triggerBBrush;
    }

    private void ClearTimer_Tick(object? sender, EventArgs e)
    {
        _clearTimer.Stop();
        if (_clearSessionId is null || _clearSessionId != _activeSessionId)
        {
            return;
        }

        _clearSessionId = null;
        _window.ClearTrail();
    }

    private void QueueRender(DispatcherPriority priority)
    {
        if (_disposed)
        {
            return;
        }

        _dispatcher.BeginInvoke(ProcessPendingTrailUpdate, priority);
    }

    private void ProcessPendingTrailUpdate()
    {
        TrailUpdate? update;
        lock (_updateSync)
        {
            update = _pendingUpdate;
            _pendingUpdate = null;
            _renderScheduled = false;
        }

        if (update is not null)
        {
            RenderTrail(update);
        }

        lock (_updateSync)
        {
            if (_pendingUpdate is null || _renderScheduled)
            {
                return;
            }

            _renderScheduled = true;
        }

        QueueRender(DispatcherPriority.Render);
    }

    private void RenderTrail(TrailUpdate update)
    {
        if (!_isEnabled)
        {
            return;
        }

        EnsureInitialized();
        if (update.SessionId < _activeSessionId)
        {
            return;
        }

        switch (update.UpdateKind)
        {
            case GestureTrailUpdateKind.Clear:
                _clearTimer.Stop();
                _clearSessionId = null;
                _window.ClearTrail();
                return;

            case GestureTrailUpdateKind.Start:
                _clearTimer.Stop();
                _clearSessionId = null;
                _window.ClearTrail();
                break;

            case GestureTrailUpdateKind.Update:
                _clearTimer.Stop();
                _clearSessionId = null;
                break;

            case GestureTrailUpdateKind.Complete:
                _clearTimer.Stop();
                _clearSessionId = update.SessionId;
                break;
        }

        _window.RenderTrail(update.Points, GetBrush(update.TriggerSlot), _trailWidth);

        if (update.UpdateKind != GestureTrailUpdateKind.Complete)
        {
            return;
        }

        if (_clearTimer.Interval <= TimeSpan.Zero)
        {
            _clearSessionId = null;
            _window.ClearTrail();
            return;
        }

        _clearTimer.Start();
    }

    private void RunOnDispatcher(Action action)
    {
        if (_disposed)
        {
            return;
        }

        if (_dispatcher.CheckAccess())
        {
            action();
            return;
        }

        _dispatcher.Invoke(action);
    }

    private static SolidColorBrush CreateFrozenBrush(MediaColor color)
    {
        var brush = new SolidColorBrush(color);
        brush.Freeze();
        return brush;
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        RunOnDispatcher(() =>
        {
            _clearTimer.Stop();
            _window.CloseOverlay();
        });

        _disposed = true;
    }

    private sealed record TrailUpdate(long SessionId, TriggerSlot TriggerSlot, IReadOnlyList<WpfPoint> Points, GestureTrailUpdateKind UpdateKind);
}

internal sealed class GestureOverlayWindow : Window
{
    private readonly Canvas _canvas;
    private readonly Polyline _trailGlowPolyline;
    private readonly Polyline _trailPolyline;
    private readonly Ellipse _cursorDot;
    private Rect _virtualBoundsInDip;
    private Rect _virtualBoundsInPixels;
    private Matrix _transformFromDevice = Matrix.Identity;

    public GestureOverlayWindow()
    {
        WindowStyle = WindowStyle.None;
        ResizeMode = ResizeMode.NoResize;
        AllowsTransparency = true;
        Background = System.Windows.Media.Brushes.Transparent;
        ShowInTaskbar = false;
        Topmost = true;
        ShowActivated = false;
        Focusable = false;
        IsHitTestVisible = false;

        _canvas = new Canvas
        {
            Background = System.Windows.Media.Brushes.Transparent,
            IsHitTestVisible = false
        };

        _trailGlowPolyline = new Polyline
        {
            IsHitTestVisible = false,
            StrokeLineJoin = PenLineJoin.Round,
            StrokeStartLineCap = PenLineCap.Round,
            StrokeEndLineCap = PenLineCap.Round,
            Opacity = 0.55,
            Visibility = Visibility.Collapsed
        };

        _trailPolyline = new Polyline
        {
            IsHitTestVisible = false,
            StrokeLineJoin = PenLineJoin.Round,
            StrokeStartLineCap = PenLineCap.Round,
            StrokeEndLineCap = PenLineCap.Round,
            Visibility = Visibility.Collapsed
        };

        _cursorDot = new Ellipse
        {
            IsHitTestVisible = false,
            Stroke = System.Windows.Media.Brushes.White,
            StrokeThickness = 1.5,
            Visibility = Visibility.Collapsed
        };

        _canvas.Children.Add(_trailGlowPolyline);
        _canvas.Children.Add(_trailPolyline);
        _canvas.Children.Add(_cursorDot);
        Content = _canvas;

        SourceInitialized += GestureOverlayWindow_SourceInitialized;
    }

    public void Prepare()
    {
        UpdateBounds();
        if (!IsVisible)
        {
            Show();
        }
    }

    public void RenderTrail(IReadOnlyList<WpfPoint> screenPoints, MediaBrush brush, double thickness)
    {
        if (screenPoints.Count == 0)
        {
            return;
        }

        UpdateBounds();

        if (!IsVisible)
        {
            Show();
        }

        var translatedPoints = new PointCollection(screenPoints.Select(ToCanvasPoint));

        var glowBrush = brush.Clone();
        glowBrush.Opacity = 0.45;
        if (glowBrush.CanFreeze)
        {
            glowBrush.Freeze();
        }

        _trailGlowPolyline.Stroke = glowBrush;
        _trailGlowPolyline.StrokeThickness = thickness + 5d;
        _trailGlowPolyline.Points = translatedPoints;
        _trailGlowPolyline.Visibility = translatedPoints.Count > 1 ? Visibility.Visible : Visibility.Collapsed;

        _trailPolyline.Stroke = brush;
        _trailPolyline.StrokeThickness = thickness;
        _trailPolyline.Points = translatedPoints;
        _trailPolyline.Visibility = translatedPoints.Count > 1 ? Visibility.Visible : Visibility.Collapsed;

        var dotSize = Math.Max(10d, thickness * 2.4d + 3d);
        var currentPoint = translatedPoints[^1];
        _cursorDot.Width = dotSize;
        _cursorDot.Height = dotSize;
        _cursorDot.Fill = brush;
        Canvas.SetLeft(_cursorDot, currentPoint.X - (dotSize / 2d));
        Canvas.SetTop(_cursorDot, currentPoint.Y - (dotSize / 2d));
        _cursorDot.Visibility = Visibility.Visible;
    }

    public void ClearTrail()
    {
        _trailGlowPolyline.Points.Clear();
        _trailGlowPolyline.Visibility = Visibility.Collapsed;
        _trailPolyline.Points.Clear();
        _trailPolyline.Visibility = Visibility.Collapsed;
        _cursorDot.Visibility = Visibility.Collapsed;
    }

    public void CloseOverlay()
    {
        Close();
    }

    private void UpdateBounds()
    {
        var virtualScreen = Forms.SystemInformation.VirtualScreen;
        UpdateTransformFromDevice();

        var topLeftInDip = TransformFromDevice(new WpfPoint(virtualScreen.Left, virtualScreen.Top));
        var bottomRightInDip = TransformFromDevice(new WpfPoint(virtualScreen.Right, virtualScreen.Bottom));
        var newBoundsInDip = new Rect(topLeftInDip, bottomRightInDip);
        var newBoundsInPixels = new Rect(virtualScreen.Left, virtualScreen.Top, virtualScreen.Width, virtualScreen.Height);

        if (newBoundsInDip == _virtualBoundsInDip && newBoundsInPixels == _virtualBoundsInPixels)
        {
            return;
        }

        _virtualBoundsInDip = newBoundsInDip;
        _virtualBoundsInPixels = newBoundsInPixels;
        Left = _virtualBoundsInDip.Left;
        Top = _virtualBoundsInDip.Top;
        Width = _virtualBoundsInDip.Width;
        Height = _virtualBoundsInDip.Height;
        _canvas.Width = _virtualBoundsInDip.Width;
        _canvas.Height = _virtualBoundsInDip.Height;
    }

    private void GestureOverlayWindow_SourceInitialized(object? sender, EventArgs e)
    {
        var handle = new WindowInteropHelper(this).Handle;
        var currentStyles = NativeMethods.GetWindowLongPtr(handle, NativeMethods.GWL_EXSTYLE);
        var updatedStyles = currentStyles.ToInt64()
                            | NativeMethods.WS_EX_TRANSPARENT
                            | NativeMethods.WS_EX_TOOLWINDOW
                            | NativeMethods.WS_EX_NOACTIVATE;

        NativeMethods.SetWindowLongPtr(handle, NativeMethods.GWL_EXSTYLE, new IntPtr(updatedStyles));
    }

    private void UpdateTransformFromDevice()
    {
        var source = PresentationSource.FromVisual(this);
        if (source?.CompositionTarget is null)
        {
            _transformFromDevice = Matrix.Identity;
            return;
        }

        _transformFromDevice = source.CompositionTarget.TransformFromDevice;
    }

    private WpfPoint TransformFromDevice(WpfPoint point)
    {
        return _transformFromDevice.Transform(point);
    }

    private WpfPoint ToCanvasPoint(WpfPoint screenPoint)
    {
        var pointInDip = TransformFromDevice(screenPoint);
        return new WpfPoint(
            pointInDip.X - _virtualBoundsInDip.Left,
            pointInDip.Y - _virtualBoundsInDip.Top);
    }
}
