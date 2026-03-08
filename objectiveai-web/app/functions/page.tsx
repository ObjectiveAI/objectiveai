"use client";

import { useState, useEffect, useRef } from "react";
import Link from "next/link";
import { Functions } from "objectiveai";
import { createPublicClient } from "../../lib/client";
import { deriveCategory, deriveDisplayName } from "../../lib/objectiveai";
import { NAV_HEIGHT_CALCULATION_DELAY_MS, STICKY_BAR_HEIGHT, STICKY_SEARCH_OVERLAP } from "../../lib/constants";
import { useResponsive } from "../../hooks/useResponsive";
import { LoadingSpinner, ErrorAlert, EmptyState } from "../../components/ui";

// Function item type for UI
interface FunctionItem {
  slug: string;
  owner: string;
  repository: string;
  commit: string;
  name: string;
  description: string;
  category: string;
  tags: string[];
}

const CATEGORIES = ["All", "Pinned", "Scoring", "Ranking", "Transformation", "Composite"];

const INITIAL_VISIBLE_COUNT = 6;
const LOAD_MORE_COUNT = 6;

export default function FunctionsPage() {
  const [functions, setFunctions] = useState<FunctionItem[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("All");
  const [sortBy, setSortBy] = useState("name");
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [pinnedFunctions, setPinnedFunctions] = useState<string[]>([]);
  const [navOffset, setNavOffset] = useState(96);
  const { isMobile, isTablet } = useResponsive();
  const [visibleCount, setVisibleCount] = useState(INITIAL_VISIBLE_COUNT);
  const searchRef = useRef<HTMLDivElement>(null);

  // Fetch functions from API
  useEffect(() => {
    async function fetchFunctions() {
      try {
        setIsLoading(true);
        setError(null);

        // Get all functions via SDK
        const client = createPublicClient();
        const result = await Functions.list(client);

        // Deduplicate by owner/repository (same function may have multiple commits)
        const uniqueFunctions = new Map<string, { owner: string; repository: string; commit: string }>();
        for (const fn of result.data) {
          const key = `${fn.owner}/${fn.repository}`;
          if (!uniqueFunctions.has(key)) {
            uniqueFunctions.set(key, fn);
          }
        }

        // Fetch details for each unique function via API route
        const functionItems: FunctionItem[] = await Promise.all(
          Array.from(uniqueFunctions.values()).map(async (fn) => {
            const slug = `${fn.owner}/${fn.repository}`;

            // Fetch full function details via SDK
            const details = await Functions.retrieve(client, "github", fn.owner, fn.repository, fn.commit);

            const category = deriveCategory(details);
            const name = deriveDisplayName(fn.repository);

            // Extract tags from repository name
            const tags = fn.repository.split("-").filter((t: string) => t.length > 2);
            if (details.type === "vector.function") tags.push("ranking");
            else tags.push("scoring");

            return {
              slug,
              owner: fn.owner,
              repository: fn.repository,
              commit: fn.commit,
              name,
              description: details.description || `${name} function`,
              category,
              tags,
            };
          })
        );

        setFunctions(functionItems);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load functions");
      } finally {
        setIsLoading(false);
      }
    }

    fetchFunctions();
  }, []);

  // Load pinned functions from localStorage
  useEffect(() => {
    const savedPinned = localStorage.getItem('pinned-functions');
    if (savedPinned) {
      setPinnedFunctions(JSON.parse(savedPinned));
    }
  }, []);

  // Dynamic sticky offset calculation based on nav height
  useEffect(() => {
    const updateOffset = () => {
      const navHeightStr = getComputedStyle(document.documentElement).getPropertyValue('--nav-height-actual');
      const navHeight = navHeightStr ? parseInt(navHeightStr) : (isMobile ? 84 : 96);
      setNavOffset(navHeight);
    };
    
    updateOffset();
    window.addEventListener('resize', updateOffset);
    const timer = setTimeout(updateOffset, NAV_HEIGHT_CALCULATION_DELAY_MS);
    return () => {
      window.removeEventListener('resize', updateOffset);
      clearTimeout(timer);
    };
  }, [isMobile]);

  // Reset visible count when filters change
  useEffect(() => {
    setVisibleCount(INITIAL_VISIBLE_COUNT);
  }, [searchQuery, selectedCategory, sortBy]);

  // Filter and sort functions
  const filteredFunctions = functions
    .filter(fn => {
      const matchesSearch = searchQuery === "" ||
        fn.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        fn.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
        fn.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase()));
      const matchesCategory = selectedCategory === "All" ||
        (selectedCategory === "Pinned" ? pinnedFunctions.includes(fn.slug) : fn.category === selectedCategory);
      return matchesSearch && matchesCategory;
    })
    .sort((a, b) => {
      if (sortBy === "name") return a.name.localeCompare(b.name);
      if (sortBy === "category") return a.category.localeCompare(b.category);
      return 0;
    });

  // Visible functions (paginated)
  const visibleFunctions = filteredFunctions.slice(0, visibleCount);
  const hasMore = visibleCount < filteredFunctions.length;

  // Safe gap from CSS variable
  const safeGap = 24;

  // Calculate sticky positions — overlap tucks search bar under nav padding
  const searchBarTop = navOffset - STICKY_SEARCH_OVERLAP;
  const sidebarTop = searchBarTop + STICKY_BAR_HEIGHT + safeGap;

  return (
    <div className="page">
      <div className="containerWide">
        {/* Header */}
        <div className="pageHeader">
          <div>
            <h1 className="heading2" style={{ marginBottom: '8px' }}>Functions</h1>
            <p style={{ fontSize: isMobile ? '15px' : '17px', color: 'var(--text-muted)' }}>
              Browse AI functions for scoring, ranking, and evaluation
            </p>
          </div>
        </div>

        {/* Sticky Search Bar Row with Filter Button */}
        <div
          ref={searchRef}
          className="stickySearchBar"
          style={{
            top: `${searchBarTop}px`,
            display: 'flex',
            alignItems: 'center',
            gap: '12px',
            marginBottom: safeGap,
          }}
        >
          {/* Filter/Sort Button - Left of Search Bar */}
          <button
            className="iconBtn"
            onClick={() => setFiltersOpen(!filtersOpen)}
            aria-label={filtersOpen ? "Close filters" : "Open filters"}
            aria-expanded={filtersOpen}
            style={{ flexShrink: 0 }}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M3 6h18M7 12h10M11 18h2" />
            </svg>
          </button>

          {/* Search Bar - Full Pill Shape */}
          <div className="searchBarPill" style={{ flex: 1 }}>
            <input
              type="text"
              placeholder="Search functions..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
            <svg className="searchIcon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="11" cy="11" r="8" />
              <path d="M21 21l-4.35-4.35" />
            </svg>
          </div>
        </div>

        {/* Layout - responsive, full width when filters collapsed */}
        <div style={{
          display: isMobile ? 'block' : (filtersOpen ? 'grid' : 'block'),
          gridTemplateColumns: filtersOpen ? (isTablet ? '220px 1fr' : '280px 1fr') : undefined,
          gap: isTablet ? '24px' : '32px',
          alignItems: 'start',
          width: '100%',
        }}>
          {/* Left Sidebar - Filters - Collapsible */}
          {!isMobile && filtersOpen && (
            <aside
              className="stickySidebar"
              style={{
                position: 'sticky',
                top: `${sidebarTop}px`,
                padding: '20px',
              }}
            >
              <h3 style={{
                fontSize: '12px',
                fontWeight: 600,
                marginBottom: '12px',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--text-muted)',
              }}>
                Category
              </h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', marginBottom: '20px' }}>
                {CATEGORIES.map(cat => (
                  <button
                    key={cat}
                    onClick={() => setSelectedCategory(cat)}
                    className={`filterChip ${selectedCategory === cat ? 'active' : ''}`}
                    style={{ 
                      textAlign: 'left', 
                      padding: '8px 14px',
                      opacity: cat === 'Pinned' && pinnedFunctions.length === 0 ? 0.5 : 1,
                    }}
                    disabled={cat === 'Pinned' && pinnedFunctions.length === 0}
                  >
                    {cat === 'Pinned' ? `Pinned (${pinnedFunctions.length})` : cat}
                  </button>
                ))}
              </div>

              <h3 style={{
                fontSize: '12px',
                fontWeight: 600,
                marginBottom: '12px',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--text-muted)',
              }}>
                Sort By
              </h3>
              <select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value)}
                className="select"
              >
                <option value="name">Name</option>
                <option value="category">Category</option>
              </select>
            </aside>
          )}

          {/* Function Cards Grid - Compact tiles */}
          <div style={{
            minHeight: '400px',
            display: 'flex',
            flexDirection: 'column',
            width: '100%',
          }}>
            {/* Only render grid when we have results */}
            {!isLoading && !error && visibleFunctions.length > 0 && (
            <>
              <div style={{
                display: 'grid',
                gridTemplateColumns: isMobile
                  ? '1fr'
                  : isTablet
                    ? 'repeat(2, 1fr)'
                    : filtersOpen
                      ? 'repeat(2, 1fr)'
                      : 'repeat(3, 1fr)',
                gap: isMobile ? '12px' : '16px',
              }}>
                {visibleFunctions.map(fn => (
                <Link
                  key={fn.slug}
                  href={`/functions/${fn.slug}`}
                  style={{ textDecoration: 'none', color: 'inherit' }}
                >
                  <div className="card" style={{
                    cursor: 'pointer',
                    height: '100%',
                    display: 'flex',
                    flexDirection: 'column',
                    position: 'relative',
                    padding: '16px',
                  }}>
                    <span className="tag" style={{ alignSelf: 'flex-start', marginBottom: '8px', fontSize: '11px', padding: '4px 10px' }}>
                      {fn.category}
                    </span>
                    <h3 style={{ fontSize: '16px', fontWeight: 600, marginBottom: '6px' }}>
                      {fn.name}
                    </h3>
                    <p className="card-description" style={{
                      fontSize: '13px',
                      lineHeight: 1.5,
                      color: 'var(--text-muted)',
                      flex: 1,
                      marginBottom: '12px',
                    }}>
                      {fn.description}
                    </p>
                    <div style={{
                      display: 'flex',
                      flexWrap: 'wrap',
                      gap: '4px',
                      marginBottom: '10px',
                    }}>
                      {fn.tags.slice(0, 2).map(tag => (
                        <span key={tag} style={{
                          fontSize: '11px',
                          padding: '3px 8px',
                          background: 'var(--border)',
                          borderRadius: '10px',
                          color: 'var(--text-muted)',
                        }}>
                          {tag}
                        </span>
                      ))}
                      {fn.tags.length > 2 && (
                        <span style={{
                          fontSize: '11px',
                          padding: '3px 8px',
                          color: 'var(--text-muted)',
                        }}>
                          +{fn.tags.length - 2}
                        </span>
                      )}
                    </div>
                    <div style={{
                      fontSize: '13px',
                      fontWeight: 600,
                      color: 'var(--accent)',
                      display: 'flex',
                      alignItems: 'center',
                      gap: '4px',
                    }}>
                      Open <span>→</span>
                    </div>
                  </div>
                </Link>
                ))}
              </div>

              {/* Load More */}
              {hasMore && (
                <button
                  onClick={() => setVisibleCount(prev => prev + LOAD_MORE_COUNT)}
                  style={{
                    display: 'block',
                    width: '100%',
                    padding: '16px',
                    marginTop: '24px',
                    background: 'none',
                    border: 'none',
                    fontSize: '14px',
                    fontWeight: 600,
                    color: 'var(--accent)',
                    cursor: 'pointer',
                    textAlign: 'center',
                    transition: 'opacity 0.2s',
                  }}
                  onMouseEnter={(e) => e.currentTarget.style.opacity = '0.7'}
                  onMouseLeave={(e) => e.currentTarget.style.opacity = '1'}
                >
                  Load more ({filteredFunctions.length - visibleCount} remaining)
                </button>
              )}
            </>
            )}

            {isLoading && (
              <LoadingSpinner fullPage message="Loading functions..." />
            )}

            {error && !isLoading && (
              <ErrorAlert title="Failed to load functions" message={error} />
            )}

            {!isLoading && !error && filteredFunctions.length === 0 && (
              <EmptyState message="No functions found matching your criteria" />
            )}
          </div>
        </div>

        {/* Mobile Filter Overlay */}
        {filtersOpen && isMobile && (
          <>
            <div
              style={{
                position: 'fixed',
                top: 0,
                left: 0,
                right: 0,
                bottom: 0,
                background: 'rgba(27, 27, 27, 0.7)',
                zIndex: 200,
              }}
              onClick={() => setFiltersOpen(false)}
            />
            <div
              role="dialog"
              aria-modal="true"
              aria-label="Filters"
              style={{
                position: 'fixed',
                bottom: 0,
                left: 0,
                right: 0,
                background: 'var(--card-bg)',
                zIndex: 201,
                padding: '24px',
                borderTopLeftRadius: '20px',
                borderTopRightRadius: '20px',
                boxShadow: '0 -4px 20px var(--shadow)',
                maxHeight: '70vh',
                overflowY: 'auto',
              }}>
              <div style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: '20px',
              }}>
                <h3 style={{ fontSize: '18px', fontWeight: 600 }}>Filters</h3>
                <button
                  onClick={() => setFiltersOpen(false)}
                  aria-label="Close filters"
                  style={{
                    background: 'none',
                    border: 'none',
                    fontSize: '24px',
                    cursor: 'pointer',
                    color: 'var(--text)',
                  }}
                >
                  ✕
                </button>
              </div>
              
              <h4 style={{
                fontSize: '12px',
                fontWeight: 600,
                marginBottom: '12px',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--text-muted)',
              }}>
                Category
              </h4>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: '8px', marginBottom: '24px' }}>
                {CATEGORIES.map(cat => (
                  <button
                    key={cat}
                    onClick={() => {
                      setSelectedCategory(cat);
                      setFiltersOpen(false);
                    }}
                    className={`filterChip ${selectedCategory === cat ? 'active' : ''}`}
                    style={{ 
                      opacity: cat === 'Pinned' && pinnedFunctions.length === 0 ? 0.5 : 1,
                    }}
                    disabled={cat === 'Pinned' && pinnedFunctions.length === 0}
                  >
                    {cat === 'Pinned' ? `Pinned (${pinnedFunctions.length})` : cat}
                  </button>
                ))}
              </div>

              <h4 style={{
                fontSize: '12px',
                fontWeight: 600,
                marginBottom: '12px',
                textTransform: 'uppercase',
                letterSpacing: '0.05em',
                color: 'var(--text-muted)',
              }}>
                Sort By
              </h4>
              <select
                value={sortBy}
                onChange={(e) => {
                  setSortBy(e.target.value);
                  setFiltersOpen(false);
                }}
                className="select"
              >
                <option value="name">Name</option>
                <option value="category">Category</option>
              </select>
            </div>
          </>
        )}

        {/* Info Card */}
        <div
          className="card"
          style={{
            padding: '24px',
            marginTop: '40px',
            background: 'var(--nav-surface)',
          }}
        >
          <h3 style={{ fontSize: '15px', fontWeight: 600, marginBottom: '12px' }}>
            What are Functions?
          </h3>
          <p style={{ fontSize: '14px', color: 'var(--text-muted)', lineHeight: 1.6, marginBottom: '12px' }}>
            Functions are composable scoring pipelines. Data in, score(s) out. Each function
            executes a list of tasks, where each task is either a Vector Completion or another Function.
          </p>
          <p style={{ fontSize: '14px', color: 'var(--text-muted)', lineHeight: 1.6 }}>
            Functions are hosted on GitHub as <code style={{ background: 'var(--card-bg)', padding: '2px 6px', borderRadius: '4px' }}>function.json</code> at
            the repository root. Reference by <code style={{ background: 'var(--card-bg)', padding: '2px 6px', borderRadius: '4px' }}>owner/repo</code> with optional commit SHA for immutability.
          </p>
        </div>
      </div>
    </div>
  );
}
