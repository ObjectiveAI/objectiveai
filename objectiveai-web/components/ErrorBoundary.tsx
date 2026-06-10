"use client";

import styles from "./ErrorBoundary.module.css";

export default function ErrorBoundary({
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <div className={styles.wrap}>
      <span>something went wrong</span>
      <button onClick={reset} className={styles.retry}>
        try again
      </button>
    </div>
  );
}
