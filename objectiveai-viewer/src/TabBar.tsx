import type { ReactElement } from "react";

export interface Tab {
  id: string;
  label: string;
}

interface TabBarProps {
  tabs: Tab[];
  activeTab: string;
  onSelect: (id: string) => void;
}

export function TabBar({ tabs, activeTab, onSelect }: TabBarProps): ReactElement {
  return (
    <nav
      role="tablist"
      style={{
        display: "flex",
        gap: "0.25rem",
        padding: "0.25rem 0.5rem",
        borderBottom: "1px solid #ccc",
        background: "#f7f7f7",
      }}
    >
      {tabs.map((tab) => (
        <button
          key={tab.id}
          role="tab"
          aria-selected={tab.id === activeTab}
          onClick={() => onSelect(tab.id)}
          style={{
            padding: "0.35rem 0.75rem",
            border: "none",
            background: tab.id === activeTab ? "#fff" : "transparent",
            borderBottom:
              tab.id === activeTab ? "2px solid #0066cc" : "2px solid transparent",
            cursor: "pointer",
            fontWeight: tab.id === activeTab ? 600 : 400,
          }}
        >
          {tab.label}
        </button>
      ))}
    </nav>
  );
}
