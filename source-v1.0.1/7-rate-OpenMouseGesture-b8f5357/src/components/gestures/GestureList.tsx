import { GestureCanvas } from "../common/GestureCanvas";
import type { GestureTemplate } from "../../types";
import "./GestureList.css";

interface GestureListProps {
  gestures: GestureTemplate[];
  selectedGesture: string | null;
  onSelect: (name: string) => void;
  onDelete: (name: string) => void;
  onAdd: () => void;
}

export function GestureList({ gestures, selectedGesture, onSelect, onDelete, onAdd }: GestureListProps) {
  return (
    <div className="gesture-list">
      {gestures.map((gesture) => (
        <div
          key={gesture.name}
          className={`gesture-item ${selectedGesture === gesture.name ? "selected" : ""}`}
          onClick={() => onSelect(gesture.name)}
        >
          <button
            className="delete-button"
            onClick={(e) => {
              e.stopPropagation();
              onDelete(gesture.name);
            }}
            aria-label="削除"
          >
            ×
          </button>
          <GestureCanvas
            points={gesture.points}
            width={80}
            height={80}
            strokeWidth={2}
          />
          <span className="gesture-name">{gesture.name}</span>
        </div>
      ))}
      <div className="gesture-item add-gesture-item" onClick={onAdd}>
        <span className="plus-icon">+</span>
      </div>
    </div>
  );
}
