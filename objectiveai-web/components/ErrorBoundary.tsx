"use client";

export default function ErrorBoundary({
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: "80px 24px",
        gap: "16px",
        fontFamily: "var(--font-mono)",
        fontSize: "11px",
        color: "var(--copper-mid)",
      }}
    >
      <span>something went wrong</span>
      <button
        onClick={reset}
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: "10px",
          color: "var(--info-dim)",
          background: "var(--ground-raised)",
          border: "1px solid var(--node-border)",
          borderRadius: "2px",
          padding: "4px 12px",
          cursor: "pointer",
        }}
      >
        try again
      </button>
    </div>
  );
}
