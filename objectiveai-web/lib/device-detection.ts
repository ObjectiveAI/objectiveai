/**
 * lib/device-detection.ts
 *
 * Detect device capabilities (not screen sizes).
 * Make rendering decisions based on what device can DO, not just how big it is.
 *
 * Philosophy: Capability-based detection, not breakpoint-based.
 */

import React from 'react';

export interface DeviceCapabilities {
  // Device classification (for logging/debugging only, not decision-making)
  sizeCategory: 'phone' | 'tablet' | 'desktop' | 'ultrawide';

  // Raw viewport dimensions
  viewportWidth: number;
  viewportHeight: number;

  // Orientation
  isPortrait: boolean;
  isLandscape: boolean;

  // Touch vs pointer input
  hasTouchSupport: boolean;
  canHover: boolean;

  // Display technology
  pixelRatio: number;
  isDarkMode: boolean;

  // Performance characteristics
  fpsCategoryTarget: 20 | 30 | 45 | 60 | 90 | 120;
  canRenderCurvedEdges: boolean;      // Only if FPS >= 45
  canRenderGlowEffects: boolean;      // Only if FPS >= 30
  canRenderParticles: boolean;        // Only if FPS >= 60 and vw > 640

  // Accessibility preferences
  prefersReducedMotion: boolean;
  prefersContrastEnhanced: boolean;
  prefersColorScheme: 'light' | 'dark';

  // Feature visibility (capability-based, not size-based)
  shouldShowBadges: boolean;          // Show if node width > 140px
  shouldCollapseSubtasks: boolean;    // Collapse if less than 900px horizontal space
  shouldUseSimplifiedLayout: boolean; // True if vw < 400 (extreme constraint)

  // Interaction preferences
  preferredInteraction: 'mouse' | 'touch' | 'keyboard';
}

export class DeviceDetection {
  private static cachedCapabilities: DeviceCapabilities | null = null;
  private static resizeObserver: (() => void) | null = null;

  /**
   * Detect complete device capability profile
   * Calls all detection methods and returns comprehensive capability object
   */
  static detectCapabilities(): DeviceCapabilities {
    // Return cached if available (invalidated on resize)
    if (this.cachedCapabilities) {
      return this.cachedCapabilities;
    }

    const vw = typeof window !== 'undefined' ? window.innerWidth : 1024;
    const vh = typeof window !== 'undefined' ? window.innerHeight : 768;
    const pixelRatio = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
    const isPortrait = vh > vw;

    // Determine FPS target based on device characteristics
    const fpsCategoryTarget = this.determineFpsTarget(vw, pixelRatio);

    // What can this device render?
    const canRenderCurvedEdges = fpsCategoryTarget >= 45;
    const canRenderGlowEffects = fpsCategoryTarget >= 30;
    const canRenderParticles = fpsCategoryTarget >= 60 && vw > 640;

    // What features should be visible?
    // NOTE: These are based on CAPABILITY, not screen size
    const shouldShowBadges = vw > 140; // Badges need 140px min node width
    const shouldCollapseSubtasks = vw < 900; // Collapse if less than 900px
    const shouldUseSimplifiedLayout = vw < 400; // Extreme constraint (very small phone)

    // Interaction capabilities
    const hasTouchSupport = this.detectTouchSupport();
    const canHover = !hasTouchSupport; // Can hover if no touch support
    const preferredInteraction = hasTouchSupport ? 'touch' : 'mouse';

    // Accessibility preferences
    const isDarkMode = typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)').matches : false;
    const prefersReducedMotion = typeof window !== 'undefined' ? window.matchMedia('(prefers-reduced-motion: reduce)').matches : false;
    const prefersContrastEnhanced = typeof window !== 'undefined' ? window.matchMedia('(prefers-contrast: more)').matches : false;

    const capabilities: DeviceCapabilities = {
      sizeCategory: this.categorizeSizeCategory(vw),
      viewportWidth: vw,
      viewportHeight: vh,
      isPortrait,
      isLandscape: !isPortrait,
      hasTouchSupport,
      canHover,
      pixelRatio,
      isDarkMode,
      fpsCategoryTarget,
      canRenderCurvedEdges,
      canRenderGlowEffects,
      canRenderParticles,
      prefersReducedMotion,
      prefersContrastEnhanced,
      prefersColorScheme: isDarkMode ? 'dark' : 'light',
      shouldShowBadges,
      shouldCollapseSubtasks,
      shouldUseSimplifiedLayout,
      preferredInteraction,
    };

    this.cachedCapabilities = capabilities;
    return capabilities;
  }

  /**
   * Determine FPS target based on device heuristics
   * Heuristic: smaller screens and lower pixel ratios get lower targets
   */
  private static determineFpsTarget(vw: number, pixelRatio: number): 20 | 30 | 45 | 60 | 90 | 120 {
    // Very small phone in extreme constraint mode
    if (vw < 400) return 20;

    // Phone (400-640px)
    if (vw < 640) return 30;

    // Tablet portrait (640-768px)
    if (vw < 800) return 45;

    // Tablet landscape / low-end laptop (800-1200px)
    if (vw < 1200) return 60;

    // Standard laptop (1200-1440px)
    if (vw < 1440) return 60;

    // High-res desktop, choose based on pixel ratio
    if (pixelRatio > 1.5) return 90; // Retina/high-dpi: 90fps

    return 120; // Standard or ultrawide: 120fps
  }

  /**
   * Categorize viewport width into size category
   * Used for logging/debugging, NOT for feature decisions
   */
  private static categorizeSizeCategory(vw: number): 'phone' | 'tablet' | 'desktop' | 'ultrawide' {
    if (vw < 640) return 'phone';
    if (vw < 1024) return 'tablet';
    if (vw < 1440) return 'desktop';
    return 'ultrawide';
  }

  /**
   * Detect touch support
   * Works on all browsers, including iPad and Android
   */
  private static detectTouchSupport(): boolean {
    if (typeof window === 'undefined') return false;

    return (
      ('ontouchstart' in window) ||
      (navigator.maxTouchPoints > 0) ||
      ((navigator as any).msMaxTouchPoints > 0)
    );
  }

  /**
   * Check if device is likely a mobile device
   * Based on touch support and viewport width
   */
  static isMobileDevice(): boolean {
    const caps = this.detectCapabilities();
    return caps.hasTouchSupport && caps.viewportWidth < 768;
  }

  /**
   * Check if device is likely a tablet
   * Based on touch support and viewport size
   */
  static isTabletDevice(): boolean {
    const caps = this.detectCapabilities();
    return caps.hasTouchSupport && caps.viewportWidth >= 640 && caps.viewportWidth < 1024;
  }

  /**
   * Invalidate cache (call on resize or media query change)
   */
  static invalidateCache(): void {
    this.cachedCapabilities = null;
  }
}

/**
 * React Hook: useDeviceCapabilities
 * Subscribe to device capability changes
 * Re-runs detection when viewport resizes or media queries change
 */
export function useDeviceCapabilities(): DeviceCapabilities {
  const [capabilities, setCapabilities] = React.useState(() =>
    DeviceDetection.detectCapabilities()
  );

  React.useEffect(() => {
    // Handle viewport resize
    const handleResize = () => {
      DeviceDetection.invalidateCache();
      setCapabilities(DeviceDetection.detectCapabilities());
    };

    window.addEventListener('resize', handleResize, { passive: true });

    // Handle media query changes (dark mode, reduced motion, contrast, orientation)
    const darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    const contrastQuery = window.matchMedia('(prefers-contrast: more)');
    const orientationQuery = window.matchMedia('(orientation: portrait)');

    const handleMediaChange = () => {
      DeviceDetection.invalidateCache();
      setCapabilities(DeviceDetection.detectCapabilities());
    };

    // Modern way: use addEventListener if available
    if (darkModeQuery.addEventListener) {
      darkModeQuery.addEventListener('change', handleMediaChange);
      reducedMotionQuery.addEventListener('change', handleMediaChange);
      contrastQuery.addEventListener('change', handleMediaChange);
      orientationQuery.addEventListener('change', handleMediaChange);

      return () => {
        window.removeEventListener('resize', handleResize);
        darkModeQuery.removeEventListener('change', handleMediaChange);
        reducedMotionQuery.removeEventListener('change', handleMediaChange);
        contrastQuery.removeEventListener('change', handleMediaChange);
        orientationQuery.removeEventListener('change', handleMediaChange);
      };
    } else {
      // Fallback for older browsers
      return () => {
        window.removeEventListener('resize', handleResize);
      };
    }
  }, []);

  return capabilities;
}

/**
 * React Hook: useIsMobile
 * Simple hook to check if device is mobile
 * Returns boolean, updates on resize
 */
export function useIsMobile(): boolean {
  const capabilities = useDeviceCapabilities();
  return capabilities.hasTouchSupport && capabilities.viewportWidth < 768;
}

/**
 * React Hook: useIsTablet
 * Simple hook to check if device is tablet
 * Returns boolean, updates on resize
 */
export function useIsTablet(): boolean {
  const capabilities = useDeviceCapabilities();
  return (
    capabilities.hasTouchSupport &&
    capabilities.viewportWidth >= 640 &&
    capabilities.viewportWidth < 1024
  );
}

/**
 * React Hook: usePrefersDarkMode
 * Subscribe to dark mode preference
 */
export function usePrefersDarkMode(): boolean {
  const capabilities = useDeviceCapabilities();
  return capabilities.isDarkMode;
}

/**
 * React Hook: usePrefersReducedMotion
 * Subscribe to reduced motion preference
 */
export function usePrefersReducedMotion(): boolean {
  const capabilities = useDeviceCapabilities();
  return capabilities.prefersReducedMotion;
}

/**
 * Utility: Get readable FPS category name
 */
export function getFpsCapacityDescription(fps: 20 | 30 | 45 | 60 | 90 | 120): string {
  const descriptions: Record<20 | 30 | 45 | 60 | 90 | 120, string> = {
    20: 'Very Limited (Extremely Constrained Device)',
    30: 'Limited (Mobile)',
    45: 'Moderate (Tablet)',
    60: 'Good (Desktop/Laptop)',
    90: 'High (High-End Desktop)',
    120: 'Very High (Ultra-Wide/Gaming)',
  };
  return descriptions[fps];
}

/**
 * Debug utility: Log device capabilities to console
 */
export function logDeviceCapabilities(): void {
  const caps = DeviceDetection.detectCapabilities();
  console.log('=== Device Capabilities ===');
  console.log(`Size Category: ${caps.sizeCategory} (${caps.viewportWidth}x${caps.viewportHeight})`);
  console.log(`Orientation: ${caps.isPortrait ? 'Portrait' : 'Landscape'}`);
  console.log(`Touch Support: ${caps.hasTouchSupport}`);
  console.log(`Pixel Ratio: ${caps.pixelRatio}`);
  console.log(`FPS Target: ${caps.fpsCategoryTarget}`);
  console.log(`Can Render: Curves=${caps.canRenderCurvedEdges}, Glow=${caps.canRenderGlowEffects}, Particles=${caps.canRenderParticles}`);
  console.log(`Dark Mode: ${caps.isDarkMode}`);
  console.log(`Reduced Motion: ${caps.prefersReducedMotion}`);
  console.log(`High Contrast: ${caps.prefersContrastEnhanced}`);
  console.log(`Features: Badges=${caps.shouldShowBadges}, Collapse=${caps.shouldCollapseSubtasks}`);
  console.log('===========================');
}

export default DeviceDetection;
