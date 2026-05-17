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
      className="flex gap-1 px-2 py-1 border-b border-node-border bg-ground-raised"
    >
      {tabs.map((tab) => {
        const active = tab.id === activeTab;
        return (
          <button
            key={tab.id}
            role="tab"
            aria-selected={active}
            onClick={() => onSelect(tab.id)}
            className={`px-3 py-1.5 rounded-sm font-mono text-xs border-b-2 transition-colors cursor-pointer ${
              active
                ? "border-copper-mid text-copper-bright font-semibold bg-ground-surface"
                : "border-transparent text-info-dim hover:text-info-mid"
            }`}
          >
            {tab.label}
          </button>
        );
      })}
    </nav>
  );
}
