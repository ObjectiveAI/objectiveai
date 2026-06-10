"use client";

import styles from "./ExploreToolbar.module.css";

export interface FilterDef {
  key: string;
  label: string;
  options: { value: string; label: string }[];
}

interface Props {
  search: string;
  onSearchChange: (value: string) => void;
  filters: FilterDef[];
  activeFilters: Record<string, string>;
  onFilterChange: (key: string, value: string) => void;
  resultCount: number;
  totalCount: number;
  placeholder: string;
}

export function ExploreToolbar({
  search,
  onSearchChange,
  filters,
  activeFilters,
  onFilterChange,
  resultCount,
  totalCount,
  placeholder,
}: Props) {
  const filtered = search || Object.values(activeFilters).some((v) => v !== "all");

  return (
    <div className={styles.toolbar}>
      <div className={styles.searchRow}>
        <input
          type="text"
          className={styles.searchInput}
          placeholder={placeholder}
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
        />
        <span className={styles.resultCount}>
          {filtered ? `${resultCount} of ${totalCount}` : totalCount}
        </span>
      </div>

      {filters.length > 0 && (
        <div className={styles.filterRow}>
          {filters.map((filter, fi) => (
            <div key={filter.key} className={styles.filterGroup}>
              <span className={styles.filterLabel}>{filter.label}</span>
              {filter.options.map((opt) => (
                <button
                  key={opt.value}
                  className={`${styles.filterPill} ${
                    (activeFilters[filter.key] ?? "all") === opt.value
                      ? styles.filterPillActive
                      : ""
                  }`}
                  onClick={() => onFilterChange(filter.key, opt.value)}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
