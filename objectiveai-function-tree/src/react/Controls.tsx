import React from "react";

interface ControlsProps {
  onZoomIn: () => void;
  onZoomOut: () => void;
  onFitToContent: () => void;
}

export function Controls({ onZoomIn, onZoomOut, onFitToContent }: ControlsProps): React.ReactElement {
  return (
    <div className="ft-controls">
      <button
        className="ft-control-btn"
        onClick={onZoomIn}
        title="Zoom in"
        aria-label="Zoom in"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M8 3v10M3 8h10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      </button>
      <button
        className="ft-control-btn"
        onClick={onZoomOut}
        title="Zoom out"
        aria-label="Zoom out"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M3 8h10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      </button>
      <button
        className="ft-control-btn"
        onClick={onFitToContent}
        title="Fit to content"
        aria-label="Fit to content"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M2 6V2h4M10 2h4v4M14 10v4h-4M6 14H2v-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>
    </div>
  );
}
