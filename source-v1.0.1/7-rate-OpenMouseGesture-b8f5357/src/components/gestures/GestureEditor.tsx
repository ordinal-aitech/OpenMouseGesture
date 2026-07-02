import { useState, useEffect } from "react";
import { Button } from "../common/Button";
import { GestureCanvas } from "../common/GestureCanvas";
import type { GestureTemplate } from "../../types";
import "./GestureEditor.css";

interface GestureEditorProps {
  gesture: GestureTemplate | null;
  isNew?: boolean;
  onSave: (name: string, points: [number, number][]) => void;
  onDelete?: () => void;
  onCancel: () => void;
}

export function GestureEditor({
  gesture,
  isNew = false,
  onSave,
  onDelete,
  onCancel,
}: GestureEditorProps) {
  const [name, setName] = useState(gesture?.name || "");
  const [points, setPoints] = useState<[number, number][]>(gesture?.points || []);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setName(gesture?.name || "");
    setPoints(gesture?.points || []);
    setError(null);
  }, [gesture]);

  const handleSave = () => {
    if (!name.trim()) {
      setError("ジェスチャー名を入力してください");
      return;
    }
    if (points.length < 10) {
      setError("ジェスチャーを描画してください");
      return;
    }
    onSave(name.trim(), points);
  };

  const handleDrawComplete = (drawnPoints: [number, number][]) => {
    setPoints(drawnPoints);
    setError(null);
  };

  return (
    <div className="gesture-editor">
      <h3 className="editor-title">
        {isNew ? "新規ジェスチャー" : "ジェスチャー編集"}
      </h3>

      <div className="editor-form">
        <div className="form-group">
          <label htmlFor="gesture-name">名前</label>
          <input
            id="gesture-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="ジェスチャー名を入力"
          />
        </div>

        <div className="form-group">
          <label>ジェスチャー描画</label>
          <GestureCanvas
            points={points}
            width={250}
            height={250}
            editable={true}
            onDrawComplete={handleDrawComplete}
          />
          <p className="form-hint">マウスでジェスチャーを描画してください</p>
        </div>

        {error && <p className="form-error">{error}</p>}

        <div className="editor-actions">
          <Button variant="ghost" onClick={onCancel}>
            キャンセル
          </Button>
          {!isNew && onDelete && (
            <Button variant="danger" onClick={onDelete}>
              削除
            </Button>
          )}
          <Button variant="primary" onClick={handleSave}>
            保存
          </Button>
        </div>
      </div>
    </div>
  );
}
