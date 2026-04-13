"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import type { ReactNode } from "react";
import styles from "./Shell.module.css";

export function Shell({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const isHome = pathname === "/";
  const isDemo = pathname === "/demo";
  const isViewerPreview = pathname === "/viewer-preview";

  return (
    <>
      {!isHome && !isDemo && !isViewerPreview && (
        <header className={styles.header}>
          <Link href="/" className={styles.logo}>
            <span className={styles.logoMark} />
            objectiveai
          </Link>
          <nav className={styles.nav}>
            <Link
              href="/explore"
              className={`${styles.navLink} ${
                pathname.startsWith("/explore") || pathname.startsWith("/functions") || pathname.startsWith("/swarms") ? styles.navLinkActive : ""
              }`}
            >
              explore
            </Link>
          </nav>
        </header>
      )}
      <main className={styles.main}>{children}</main>
    </>
  );
}
