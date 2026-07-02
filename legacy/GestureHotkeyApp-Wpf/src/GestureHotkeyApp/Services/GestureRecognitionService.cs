using WpfPoint = System.Windows.Point;

namespace GestureHotkeyApp.Services;

public class GestureRecognitionService
{
    private const double MinimumTotalDistance = 40.0;
    private const double MinimumSegmentDistance = 12.0;

    public string? Recognize(IReadOnlyList<WpfPoint> points)
    {
        if (points.Count < 2)
        {
            return null;
        }

        var totalDistance = 0.0;
        for (var i = 1; i < points.Count; i++)
        {
            totalDistance += Distance(points[i - 1], points[i]);
        }

        if (totalDistance < MinimumTotalDistance)
        {
            return null;
        }

        var directions = new List<string>();
        var lastAnchor = points[0];

        for (var i = 1; i < points.Count; i++)
        {
            var current = points[i];
            var distance = Distance(lastAnchor, current);
            if (distance < MinimumSegmentDistance)
            {
                continue;
            }

            var direction = ToDirection(lastAnchor, current);
            if (directions.Count == 0 || !string.Equals(directions[^1], direction, StringComparison.Ordinal))
            {
                directions.Add(direction);
            }

            lastAnchor = current;
        }

        return directions.Count == 0 ? null : string.Join('-', directions);
    }

    private static double Distance(WpfPoint left, WpfPoint right)
    {
        var dx = right.X - left.X;
        var dy = right.Y - left.Y;
        return Math.Sqrt(dx * dx + dy * dy);
    }

    private static string ToDirection(WpfPoint from, WpfPoint to)
    {
        var dx = to.X - from.X;
        var dy = to.Y - from.Y;
        var angle = Math.Atan2(dy, dx) * 180.0 / Math.PI;
        if (angle < 0)
        {
            angle += 360.0;
        }

        if (angle is >= 337.5 or < 22.5) return "R";
        if (angle < 67.5) return "DR";
        if (angle < 112.5) return "D";
        if (angle < 157.5) return "DL";
        if (angle < 202.5) return "L";
        if (angle < 247.5) return "UL";
        if (angle < 292.5) return "U";
        return "UR";
    }
}
