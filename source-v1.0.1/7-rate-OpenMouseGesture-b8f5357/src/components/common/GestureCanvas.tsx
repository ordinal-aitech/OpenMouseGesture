import { useRef, useEffect, useState, useCallback } from "react";
import "./GestureCanvas.css";

interface GestureCanvasProps {
  points?: [number, number][];
  width?: number;
  height?: number;
  editable?: boolean;
  onDrawComplete?: (points: [number, number][]) => void;
  strokeColor?: string;
  strokeWidth?: number;
}

export function GestureCanvas({
  points = [],
  width = 200,
  height = 200,
  editable = false,
  onDrawComplete,
  strokeColor = "#5A7863",
  strokeWidth = 3,
}: GestureCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [isDrawing, setIsDrawing] = useState(false);
  const [drawnPoints, setDrawnPoints] = useState<[number, number][]>([]);

  const normalizePoints = useCallback(
    (pts: [number, number][]): [number, number][] => {
      if (pts.length === 0) return [];

      const xs = pts.map((p) => p[0]);
      const ys = pts.map((p) => p[1]);
      const minX = Math.min(...xs);
      const maxX = Math.max(...xs);
      const minY = Math.min(...ys);
      const maxY = Math.max(...ys);

      const rangeX = maxX - minX || 1;
      const rangeY = maxY - minY || 1;
      const margin = width * 0.3;
      const scale = Math.min((width - margin) / rangeX, (height - margin) / rangeY);

      const centerX = (minX + maxX) / 2;
      const centerY = (minY + maxY) / 2;

      return pts.map((p) => [
        (p[0] - centerX) * scale + width / 2,
        (p[1] - centerY) * scale + height / 2,
      ]);
    },
    [width, height]
  );

  const drawPath = useCallback(
    (ctx: CanvasRenderingContext2D, pts: [number, number][]) => {
      if (pts.length < 2) return;

      ctx.beginPath();
      ctx.strokeStyle = strokeColor;
      ctx.lineWidth = strokeWidth;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";

      ctx.moveTo(pts[0][0], pts[0][1]);
      for (let i = 1; i < pts.length; i++) {
        ctx.lineTo(pts[i][0], pts[i][1]);
      }
      ctx.stroke();

      ctx.fillStyle = "#3B4953";
      ctx.beginPath();
      ctx.arc(pts[0][0], pts[0][1], strokeWidth * 2, 0, 2 * Math.PI);
      ctx.fill();

      if (pts.length > 1) {
        const lastIdx = pts.length - 1;
        const prevIdx = Math.max(0, lastIdx - 5);
        const dx = pts[lastIdx][0] - pts[prevIdx][0];
        const dy = pts[lastIdx][1] - pts[prevIdx][1];
        const angle = Math.atan2(dy, dx);

        const arrowSize = strokeWidth * 2;
        ctx.save();
        ctx.translate(pts[lastIdx][0], pts[lastIdx][1]);
        ctx.rotate(angle);

        ctx.fillStyle = strokeColor;
        ctx.beginPath();
        ctx.moveTo(arrowSize, 0);
        ctx.lineTo(-arrowSize, -arrowSize);
        ctx.lineTo(-arrowSize, arrowSize);
        ctx.closePath();
        ctx.fill();
        ctx.restore();
      }
    },
    [strokeColor, strokeWidth]
  );

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    ctx.clearRect(0, 0, width, height);

    if (editable && drawnPoints.length > 0) {
      drawPath(ctx, drawnPoints);
    } else if (points.length > 0) {
      const normalized = normalizePoints(points);
      drawPath(ctx, normalized);
    }
  }, [points, drawnPoints, editable, width, height, normalizePoints, drawPath]);

  const getCanvasCoordinates = (
    e: React.MouseEvent<HTMLCanvasElement>
  ): [number, number] => {
    const canvas = canvasRef.current;
    if (!canvas) return [0, 0];

    const rect = canvas.getBoundingClientRect();
    return [e.clientX - rect.left, e.clientY - rect.top];
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!editable) return;
    setIsDrawing(true);
    const point = getCanvasCoordinates(e);
    setDrawnPoints([point]);
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!editable || !isDrawing) return;
    const point = getCanvasCoordinates(e);
    setDrawnPoints((prev) => [...prev, point]);
  };

  const handleMouseUp = () => {
    if (!editable || !isDrawing) return;
    setIsDrawing(false);
    if (drawnPoints.length >= 10 && onDrawComplete) {
      onDrawComplete(drawnPoints);
    }
  };

  const handleMouseLeave = () => {
    if (isDrawing) {
      handleMouseUp();
    }
  };

  const handleClear = () => {
    setDrawnPoints([]);
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, width, height);
  };

  return (
    <div className="gesture-canvas-container">
      <canvas
        ref={canvasRef}
        width={width}
        height={height}
        className={`gesture-canvas ${editable ? "editable" : ""}`}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
      />
      {editable && drawnPoints.length > 0 && (
        <button className="canvas-clear-btn" onClick={handleClear}>
          クリア
        </button>
      )}
    </div>
  );
}
