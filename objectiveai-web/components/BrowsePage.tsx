"use client";

import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useRef,
  ReactNode,
} from "react";
import { useIsMobile } from "../hooks/useIsMobile";
import { NAV_HEIGHT_CALCULATION_DELAY_MS, STICKY_BAR_HEIGHT, SAFE_GAP } from "../lib/constants";

// ============================================================================
// Context
// ============================================================================

interface BrowsePageContextValue {
  // Responsive state
  isMobile: boolean;
  isTablet: boolean;

  // Filter panel state
  filtersOpen: boolean;
  setFiltersOpen: (open: boolean) => void;

  // Sticky positioning
  navOffset: number;
  searchBarTop: number;
  sidebarTop: number;

  // Search state (lifted for filter resets)
  searchQuery: string;
  setSearchQuery: (query: string) => void;
}

const BrowsePageContext = createContext<BrowsePageContextValue | null>(null);

function useBrowsePageContext() {
  const context = useContext(BrowsePageContext);
  if (!context) {
    throw new Error("BrowsePage compound components must be used within BrowsePage");
  }
  return context;
}

// ============================================================================
// Root Component
// ============================================================================

interface BrowsePageProps {
  children: ReactNode;
  /** Optional: Override the default max-width (1400px) */
  maxWidth?: string;
}

function BrowsePageRoot({ children, maxWidth = "1400px" }: BrowsePageProps) {
  const isMobile = useIsMobile();
  const [isTablet, setIsTablet] = useState(false);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [navOffset, setNavOffset] = useState(96);
  const [searchQuery, setSearchQuery] = useState("");

  // Track tablet viewport size
  useEffect(() => {
    const checkViewport = () => {
      setIsTablet(window.innerWidth <= 1024);
    };
    checkViewport();
    window.addEventListener("resize", checkViewport);
    return () => window.removeEventListener("resize", checkViewport);
  }, []);

  // Dynamic sticky offset calculation based on nav height
  useEffect(() => {
    const updateOffset = () => {
      const navHeightStr = getComputedStyle(
        document.documentElement
      ).getPropertyValue("--nav-height-actual");
      const navHeight = navHeightStr
        ? parseInt(navHeightStr)
        : isMobile
        ? 84
        : 96;
      setNavOffset(navHeight);
    };

    updateOffset();
    window.addEventListener("resize", updateOffset);
    const timer = setTimeout(updateOffset, NAV_HEIGHT_CALCULATION_DELAY_MS);
    return () => {
      window.removeEventListener("resize", updateOffset);
      clearTimeout(timer);
    };
  }, [isMobile]);

  const searchBarTop = navOffset;
  const sidebarTop = navOffset + STICKY_BAR_HEIGHT + SAFE_GAP;

  const contextValue: BrowsePageContextValue = {
    isMobile,
    isTablet,
    filtersOpen,
    setFiltersOpen,
    navOffset,
    searchBarTop,
    sidebarTop,
    searchQuery,
    setSearchQuery,
  };

  return (
    <BrowsePageContext.Provider value={contextValue}>
      <div className="page">
        <div
          style={{
            width: "100%",
            maxWidth,
            marginLeft: "auto",
            marginRight: "auto",
            padding: isMobile ? "0 16px" : "0 32px",
            boxSizing: "border-box",
          }}
        >
          {children}
        </div>
      </div>
    </BrowsePageContext.Provider>
  );
}

// ============================================================================
// Header Component
// ============================================================================

interface HeaderProps {
  title: string;
  description: string;
  /** Optional action button (e.g., "Create New") to render in header */
  action?: ReactNode;
}

function Header({ title, description, action }: HeaderProps) {
  const { isMobile } = useBrowsePageContext();

  return (
    <div
      style={{
        marginBottom: isMobile ? "24px" : "32px",
        display: action ? "flex" : "block",
        flexDirection: isMobile ? "column" : "row",
        justifyContent: "space-between",
        alignItems: isMobile ? "flex-start" : "flex-end",
        gap: isMobile ? "16px" : "24px",
      }}
    >
      <div>
        <h1 className="heading2" style={{ marginBottom: "8px" }}>
          {title}
        </h1>
        <p
          style={{
            fontSize: isMobile ? "15px" : "17px",
            color: "var(--text-muted)",
          }}
        >
          {description}
        </p>
      </div>
      {action}
    </div>
  );
}

// ============================================================================
// Search Bar Component
// ============================================================================

interface SearchBarProps {
  placeholder?: string;
  /** Optional controlled value (defaults to internal context state) */
  value?: string;
  /** Optional onChange handler (defaults to internal context setter) */
  onChange?: (value: string) => void;
}

function SearchBar({ placeholder = "Search...", value, onChange }: SearchBarProps) {
  const {
    filtersOpen,
    setFiltersOpen,
    searchBarTop,
    searchQuery,
    setSearchQuery
  } = useBrowsePageContext();

  const searchRef = useRef<HTMLDivElement>(null);

  // Use controlled props if provided, otherwise use context state
  const currentValue = value !== undefined ? value : searchQuery;
  const handleChange = onChange || setSearchQuery;

  return (
    <div
      ref={searchRef}
      style={{
        position: "sticky",
        top: `${searchBarTop}px`,
        zIndex: 100,
        display: "flex",
        alignItems: "center",
        gap: "12px",
        marginBottom: SAFE_GAP,
        background: "var(--page-bg)",
        padding: "8px 0",
      }}
    >
      {/* Filter/Sort Button - Left of Search Bar */}
      <button
        className="iconBtn"
        onClick={() => setFiltersOpen(!filtersOpen)}
        aria-label={filtersOpen ? "Close filters" : "Open filters"}
        style={{ flexShrink: 0 }}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <path d="M3 6h18M7 12h10M11 18h2" />
        </svg>
      </button>

      {/* Search Bar - Full Pill Shape */}
      <div className="searchBarPill" style={{ flex: 1 }}>
        <input
          type="text"
          placeholder={placeholder}
          value={currentValue}
          onChange={(e) => handleChange(e.target.value)}
        />
        <svg
          className="searchIcon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="11" cy="11" r="8" />
          <path d="M21 21l-4.35-4.35" />
        </svg>
      </div>
    </div>
  );
}

// ============================================================================
// Layout Component (Sidebar + Content)
// ============================================================================

interface LayoutProps {
  children: ReactNode;
}

function Layout({ children }: LayoutProps) {
  const { isMobile, isTablet, filtersOpen } = useBrowsePageContext();

  return (
    <div
      style={{
        display: isMobile ? "block" : filtersOpen ? "grid" : "block",
        gridTemplateColumns: filtersOpen
          ? isTablet
            ? "220px 1fr"
            : "280px 1fr"
          : undefined,
        gap: isTablet ? "24px" : "32px",
        alignItems: "start",
        width: "100%",
      }}
    >
      {children}
    </div>
  );
}

// ============================================================================
// Filters Component (Desktop Sidebar)
// ============================================================================

interface FiltersProps {
  children: ReactNode;
}

function Filters({ children }: FiltersProps) {
  const { isMobile, filtersOpen, sidebarTop } = useBrowsePageContext();

  // Don't render on mobile or when filters are closed
  if (isMobile || !filtersOpen) {
    return null;
  }

  return (
    <aside
      className="stickySidebar"
      style={{
        position: "sticky",
        top: `${sidebarTop}px`,
        padding: "20px",
      }}
    >
      {children}
    </aside>
  );
}

// ============================================================================
// Filter Section Component (for consistent styling within Filters)
// ============================================================================

interface FilterSectionProps {
  title: string;
  children: ReactNode;
  /** Add bottom margin (default: true) */
  withMargin?: boolean;
}

function FilterSection({ title, children, withMargin = true }: FilterSectionProps) {
  return (
    <>
      <h3
        style={{
          fontSize: "12px",
          fontWeight: 600,
          marginBottom: "12px",
          textTransform: "uppercase",
          letterSpacing: "0.05em",
          color: "var(--text-muted)",
        }}
      >
        {title}
      </h3>
      <div style={{ marginBottom: withMargin ? "20px" : 0 }}>{children}</div>
    </>
  );
}

// ============================================================================
// Content Component (Main grid/list area)
// ============================================================================

interface ContentProps {
  children: ReactNode;
}

function Content({ children }: ContentProps) {
  return (
    <div
      style={{
        minHeight: "400px",
        display: "flex",
        flexDirection: "column",
        width: "100%",
      }}
    >
      {children}
    </div>
  );
}

// ============================================================================
// Grid Component
// ============================================================================

interface GridProps {
  children: ReactNode;
  /** Number of columns on desktop when filters closed (default: 3) */
  columns?: number;
  /** Number of columns on desktop when filters open (default: 2) */
  columnsWithFilters?: number;
  /** Grid layout or list layout */
  layout?: "grid" | "list";
}

function Grid({
  children,
  columns = 3,
  columnsWithFilters = 2,
  layout = "grid",
}: GridProps) {
  const { isMobile, isTablet, filtersOpen } = useBrowsePageContext();

  if (layout === "list") {
    return (
      <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
        {children}
      </div>
    );
  }

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: isMobile
          ? "1fr"
          : isTablet
          ? "repeat(2, 1fr)"
          : filtersOpen
          ? `repeat(${columnsWithFilters}, 1fr)`
          : `repeat(${columns}, 1fr)`,
        gap: isMobile ? "12px" : "16px",
      }}
    >
      {children}
    </div>
  );
}

// ============================================================================
// Load More Component
// ============================================================================

interface LoadMoreProps {
  onClick: () => void;
  remaining: number;
  /** Whether there are more items to load */
  hasMore: boolean;
}

function LoadMore({ onClick, remaining, hasMore }: LoadMoreProps) {
  if (!hasMore) {
    return null;
  }

  return (
    <button
      onClick={onClick}
      style={{
        display: "block",
        width: "100%",
        padding: "16px",
        marginTop: "24px",
        background: "none",
        border: "none",
        fontSize: "14px",
        fontWeight: 600,
        color: "var(--accent)",
        cursor: "pointer",
        textAlign: "center",
        transition: "opacity 0.2s",
      }}
      onMouseEnter={(e) => (e.currentTarget.style.opacity = "0.7")}
      onMouseLeave={(e) => (e.currentTarget.style.opacity = "1")}
    >
      Load more ({remaining} remaining)
    </button>
  );
}

// ============================================================================
// State Components (Loading, Error, Empty)
// ============================================================================

interface LoadingProps {
  message?: string;
}

function Loading({ message = "Loading..." }: LoadingProps) {
  return (
    <div
      style={{
        textAlign: "center",
        padding: "60px 20px",
        color: "var(--text-muted)",
        flex: 1,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <div
        className="spinner"
        style={{
          width: "40px",
          height: "40px",
          border: "3px solid var(--border)",
          borderTopColor: "var(--accent)",
          borderRadius: "50%",
          margin: "0 auto 16px",
          animation: "spin 1s linear infinite",
        }}
      />
      <p>{message}</p>
    </div>
  );
}

interface ErrorStateProps {
  message: string;
  title?: string;
}

function ErrorState({ message, title = "Failed to load" }: ErrorStateProps) {
  return (
    <div
      style={{
        textAlign: "center",
        padding: "60px 20px",
        color: "var(--text-muted)",
        flex: 1,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <p style={{ color: "var(--color-error)", marginBottom: "8px" }}>{title}</p>
      <p style={{ fontSize: "14px" }}>{message}</p>
    </div>
  );
}

interface EmptyProps {
  message: string;
}

function Empty({ message }: EmptyProps) {
  return (
    <div
      style={{
        textAlign: "center",
        padding: "60px 20px",
        color: "var(--text-muted)",
        flex: 1,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      {message}
    </div>
  );
}

// ============================================================================
// Mobile Filter Overlay Component
// ============================================================================

interface MobileFiltersProps {
  children: ReactNode;
  /** Title for the mobile overlay (default: "Filters") */
  title?: string;
}

function MobileFilters({ children, title = "Filters" }: MobileFiltersProps) {
  const { isMobile, filtersOpen, setFiltersOpen } = useBrowsePageContext();

  // Only render on mobile when filters are open
  if (!isMobile || !filtersOpen) {
    return null;
  }

  return (
    <>
      {/* Backdrop */}
      <div
        style={{
          position: "fixed",
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: "rgba(27, 27, 27, 0.7)",
          zIndex: 200,
        }}
        onClick={() => setFiltersOpen(false)}
      />
      {/* Bottom Sheet */}
      <div
        style={{
          position: "fixed",
          bottom: 0,
          left: 0,
          right: 0,
          background: "var(--card-bg)",
          zIndex: 201,
          padding: "24px",
          borderTopLeftRadius: "20px",
          borderTopRightRadius: "20px",
          boxShadow: "0 -4px 20px var(--shadow)",
          maxHeight: "70vh",
          overflowY: "auto",
        }}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: "20px",
          }}
        >
          <h3 style={{ fontSize: "18px", fontWeight: 600 }}>{title}</h3>
          <button
            onClick={() => setFiltersOpen(false)}
            style={{
              background: "none",
              border: "none",
              fontSize: "24px",
              cursor: "pointer",
              color: "var(--text)",
            }}
            aria-label="Close filters"
          >
            &#x2715;
          </button>
        </div>
        {/* Filter Content */}
        {children}
      </div>
    </>
  );
}

// ============================================================================
// Mobile Filter Section Component
// ============================================================================

interface MobileFilterSectionProps {
  title: string;
  children: ReactNode;
  /** Add bottom margin (default: true) */
  withMargin?: boolean;
}

function MobileFilterSection({ title, children, withMargin = true }: MobileFilterSectionProps) {
  return (
    <>
      <h4
        style={{
          fontSize: "12px",
          fontWeight: 600,
          marginBottom: "12px",
          textTransform: "uppercase",
          letterSpacing: "0.05em",
          color: "var(--text-muted)",
        }}
      >
        {title}
      </h4>
      <div style={{ marginBottom: withMargin ? "24px" : 0 }}>{children}</div>
    </>
  );
}

// ============================================================================
// Info Card Component (for explanatory cards at bottom of page)
// ============================================================================

interface InfoCardProps {
  title: string;
  children: ReactNode;
}

function InfoCard({ title, children }: InfoCardProps) {
  return (
    <div
      className="card"
      style={{
        padding: "24px",
        marginTop: "40px",
        background: "var(--nav-surface)",
      }}
    >
      <h3 style={{ fontSize: "15px", fontWeight: 600, marginBottom: "12px" }}>
        {title}
      </h3>
      <div style={{ fontSize: "14px", color: "var(--text-muted)", lineHeight: 1.6 }}>
        {children}
      </div>
    </div>
  );
}

// ============================================================================
// Exports (Compound Component Pattern)
// ============================================================================

export const BrowsePage = Object.assign(BrowsePageRoot, {
  Header,
  SearchBar,
  Layout,
  Filters,
  FilterSection,
  Content,
  Grid,
  LoadMore,
  Loading,
  ErrorState,
  Empty,
  MobileFilters,
  MobileFilterSection,
  InfoCard,
});

// Export context hook for advanced use cases
export { useBrowsePageContext };

// Export types for consumers
export type { BrowsePageContextValue };
