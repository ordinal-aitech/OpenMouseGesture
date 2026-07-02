import type { TabId } from "../../types";
import "./Tabs.css";

interface TabsProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
}

const tabs: { id: TabId; label: string }[] = [
  { id: "gestures", label: "ジェスチャー" },
  { id: "actions", label: "アクション" },
  { id: "settings", label: "設定" },
  { id: "licenses", label: "ライセンス" },
  { id: "info", label: "情報" },
];

export function Tabs({ activeTab, onTabChange }: TabsProps) {
  return (
    <div className="tabs">
      {tabs.map((tab) => (
        <button
          key={tab.id}
          className={`tab ${activeTab === tab.id ? "active" : ""}`}
          onClick={() => onTabChange(tab.id)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
