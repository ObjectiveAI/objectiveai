"use client";

import { useState, useEffect } from "react";
import Link from "next/link";
import { Functions } from "objectiveai";
import { createPublicClient } from "../lib/client";
import HeroText from "@/components/HeroText";
import { deriveCategory, deriveDisplayName } from "../lib/objectiveai";
import { useResponsive } from "../hooks/useResponsive";

// =============================================================================
// FEATURED FUNCTIONS CONFIGURATION
// -----------------------------------------------------------------------------
// Number of functions to display on the landing page.
// Functions are fetched from the API and the first N are shown.
// To feature specific functions, they can be pinned in the functions page.
// =============================================================================
const FEATURED_COUNT = 3;

interface FeaturedFunction {
  slug: string;
  name: string;
  description: string;
  category: string;
  tags: string[];
}

export default function Home() {
  const { isMobile } = useResponsive();
  const [functions, setFunctions] = useState<FeaturedFunction[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  // Fetch functions from API
  useEffect(() => {
    async function fetchFunctions() {
      try {
        setIsLoading(true);

        // Fetch functions list via SDK
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

        // Limit to FEATURED_COUNT
        const limitedFunctions = Array.from(uniqueFunctions.values()).slice(0, FEATURED_COUNT);

        const functionItems: FeaturedFunction[] = await Promise.all(
          limitedFunctions.map(async (fn) => {
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
              name,
              description: details.description || `${name} function`,
              category,
              tags,
            };
          })
        );

        setFunctions(functionItems);
      } catch {
        // Silent failure - page still renders, just without featured functions
      } finally {
        setIsLoading(false);
      }
    }

    fetchFunctions();
  }, []);

  return (
    <div className="page" style={{
      display: 'flex',
      flexDirection: 'column',
      gap: isMobile ? '80px' : '120px',
      paddingBottom: '60px',
    }}>
      {/* Hero Section */}
      <section style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: 'calc(45vh - 100px)',
        paddingTop: isMobile ? '32px' : '48px',
      }}>
        <div style={{ textAlign: 'center', display: 'flex', flexDirection: 'column', alignItems: 'center', width: '100%', padding: isMobile ? '0 16px' : '0 32px', maxWidth: '800px' }}>
          <p style={{
            fontSize: isMobile ? '11px' : '13px',
            fontWeight: 600,
            textTransform: 'uppercase',
            letterSpacing: '0.12em',
            color: 'var(--accent)',
            marginBottom: '12px',
          }}>
            AI Scoring Primitives for Developers
          </p>
          <div style={{ marginBottom: '12px', width: '100%' }}>
            <HeroText />
          </div>
          <p style={{
            fontSize: isMobile ? '14px' : '17px',
            color: 'var(--text-muted)',
            marginBottom: '24px',
            maxWidth: '395px',
            textWrap: 'balance',
          }}>
            Ensembles of LLMs, voting, to provide confidence in objective AI measurements.
          </p>
          <div style={{
            display: 'flex',
            gap: '12px',
            justifyContent: 'center',
            flexWrap: 'wrap',
          }}>
            <Link href="/functions" className="pillBtn">
              Functions
            </Link>
            <a
              href="https://github.com/ObjectiveAI/objectiveai"
              target="_blank"
              rel="noopener noreferrer"
              className="pillBtn"
            >
              GitHub
            </a>
          </div>
        </div>
      </section>

      {/* Featured Functions Section */}
      <section>
        <div className="container">
          <div style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'flex-end',
            marginBottom: isMobile ? '24px' : '32px',
            flexWrap: 'wrap',
            gap: '16px',
          }}>
            <div>
              <span className="tag" style={{ marginBottom: '12px', display: 'inline-block' }}>
                Explore
              </span>
              <h2 className="heading2">Featured Functions</h2>
            </div>
            <Link
              href="/functions"
              style={{
                fontSize: '15px',
                fontWeight: 600,
                color: 'var(--accent)',
                textDecoration: 'none',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
              }}
            >
              View all <span>→</span>
            </Link>
          </div>

          {/* Function Cards Grid */}
          <div className="gridThree">
            {isLoading ? (
              // Loading skeleton
              Array.from({ length: FEATURED_COUNT }).map((_, i) => (
                <div key={i} className="card" style={{
                  padding: '16px',
                  height: '180px',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '8px',
                }}>
                  <div style={{
                    width: '60px',
                    height: '20px',
                    background: 'var(--border)',
                    borderRadius: '10px',
                    animation: 'pulse 1.5s ease-in-out infinite',
                  }} />
                  <div style={{
                    width: '80%',
                    height: '18px',
                    background: 'var(--border)',
                    borderRadius: '4px',
                    animation: 'pulse 1.5s ease-in-out infinite',
                  }} />
                  <div style={{
                    width: '100%',
                    height: '32px',
                    background: 'var(--border)',
                    borderRadius: '4px',
                    animation: 'pulse 1.5s ease-in-out infinite',
                  }} />
                </div>
              ))
            ) : functions.length > 0 ? (
              functions.map(fn => (
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
                    <span className="tag" style={{
                      alignSelf: 'flex-start',
                      marginBottom: '8px',
                      fontSize: '11px',
                      padding: '4px 10px'
                    }}>
                      {fn.category}
                    </span>
                    <h3 style={{ fontSize: '16px', fontWeight: 600, marginBottom: '6px' }}>
                      {fn.name}
                    </h3>
                    <p style={{
                      fontSize: '13px',
                      lineHeight: 1.5,
                      color: 'var(--text-muted)',
                      flex: 1,
                      marginBottom: '12px',
                      display: '-webkit-box',
                      WebkitLineClamp: 2,
                      WebkitBoxOrient: 'vertical',
                      overflow: 'hidden',
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
              ))
            ) : (
              // Empty state
              <div style={{
                gridColumn: '1 / -1',
                textAlign: 'center',
                padding: '48px 24px',
                color: 'var(--text-muted)',
              }}>
                <p>No functions available yet.</p>
                <Link
                  href="/functions"
                  style={{
                    color: 'var(--accent)',
                    textDecoration: 'none',
                    fontWeight: 500,
                  }}
                >
                  Browse all functions →
                </Link>
              </div>
            )}
          </div>
        </div>
      </section>

    </div>
  );
}
