"use client";

import cn from "classnames";
import {
  ReactElement,
  RefObject,
  ReactNode,
  useEffect,
  useRef,
  CSSProperties,
} from "react";
import { createPortal } from "react-dom";
import { autoUpdate, Placement, useFloating } from "@floating-ui/react";

export function ConfirmPopup({
  className,
  onConfirm,
  onCancel,
  title,
  body,
  canConfirm = true,
}: {
  className?: string;
  onConfirm?: () => void;
  onCancel?: () => void;
  title: ReactNode;
  body: ReactNode;
  canConfirm?: boolean;
}): ReactElement {
  return createPortal(
    <Popup<HTMLDivElement> className={cn(className)}>
      {(ref) => (
        <div
          ref={ref as RefObject<HTMLDivElement>}
          className={cn(
            "flex",
            "flex-col",
            "space-y-2",
            "border",
            "border-muted-secondary",
            "rounded-lg",
            "p-4",
            "[background-color:rgb(9,0,20)]",
            "space-y-4"
          )}
        >
          <div className={cn("text-xl", "font-bold")}>{title}</div>
          {body}
          <div className={cn("flex", "justify-between")}>
            <button
              className={cn(
                "px-4",
                "py-2",
                "text-primary",
                "text-sm",
                "border",
                "border-muted-secondary",
                "hover:border-white",
                "rounded-lg",
                "hover:bg-highlight-secondary",
                "cursor-pointer"
              )}
              onClick={onCancel}
            >
              Cancel
            </button>
            <button
              className={cn(
                "px-4",
                "py-2",
                canConfirm
                  ? cn(
                      "text-primary",
                      "hover:border-white",
                      "hover:bg-highlight-secondary",
                      "cursor-pointer"
                    )
                  : cn(
                      "text-muted-primary",
                      "hover:bg-highlight-muted",
                      "cursor-not-allowed"
                    ),
                "text-sm",
                "border",
                "border-muted-secondary",
                "rounded-lg"
              )}
              onClick={canConfirm ? onConfirm : undefined}
            >
              Confirm
            </button>
          </div>
        </div>
      )}
    </Popup>,
    document.body
  );
}

export function Popup<T extends HTMLElement>({
  className,
  // onCancel,
  children,
}: {
  className?: string;
  // onCancel?: () => void;
  children?: (ref: RefObject<T | null>) => ReactNode;
}): ReactElement {
  const popupRef = useRef<T>(null);
  // useEffect(() => {
  //   const handleClick = (e: MouseEvent | TouchEvent) => {
  //     if (popupRef.current && !popupRef.current.contains(e.target as Node)) {
  //       onCancel?.();
  //     }
  //   };
  //   window.addEventListener("pointerdown", handleClick, { passive: true });
  //   return () => window.removeEventListener("pointerdown", handleClick);
  // }, [onCancel]);
  return (
    <div
      className={cn(
        "fixed",
        "inset-0",
        "z-50",
        "flex",
        "items-center",
        "justify-center",
        "bg-black/80",
        "transition-opacity",
        "duration-200",
        className
      )}
    >
      {children?.(popupRef)}
    </div>
  );
}

export function Dropdown<RT extends HTMLElement = HTMLElement>({
  maxWidth,
  maxHeight,
  className,
  placement,
  open = false,
  onClose,
  referenceChildren,
  dropdownChildren,
}: {
  maxWidth?: number | string;
  maxHeight?: number | string;
  className?: string;
  placement: Placement;
  open?: boolean;
  onClose?: () => void;
  referenceChildren: (props: { ref: (node: RT | null) => void }) => ReactNode;
  dropdownChildren?: ReactNode;
}): ReactElement {
  const {
    refs: {
      setReference: setFloatingReferenceRef,
      floating: dropdownRef,
      setFloating: setDropdownRef,
    },
    floatingStyles,
  } = useFloating<RT>({
    placement,
    whileElementsMounted: autoUpdate,
    open,
    transform: false,
    strategy: "fixed",
  });
  const referenceRef = useRef<RT | null>(null);
  const setReferenceRef = (node: RT | null) => {
    referenceRef.current = node;
    setFloatingReferenceRef(node);
  };

  return (
    <>
      {referenceChildren({ ref: setReferenceRef })}
      {open && (
        <DropdownPopup
          maxWidth={maxWidth}
          maxHeight={maxHeight}
          referenceRef={referenceRef}
          ref={dropdownRef}
          setRef={setDropdownRef}
          style={floatingStyles}
          onClose={onClose}
          placement={placement}
          className={cn(className)}
        >
          {dropdownChildren}
        </DropdownPopup>
      )}
    </>
  );
}

function DropdownPopup({
  maxWidth,
  maxHeight,
  referenceRef,
  ref,
  setRef,
  style,
  onClose,
  placement,
  className,
  children,
}: {
  maxWidth?: number | string;
  maxHeight?: number | string;
  referenceRef: RefObject<HTMLElement | null>;
  maxHeightRef?: RefObject<HTMLElement | null>;
  ref: RefObject<HTMLElement | null>;
  setRef: (node: HTMLElement | null) => void;
  style: CSSProperties;
  onClose?: () => void;
  placement: Placement;
  className?: string;
  children?: ReactNode;
}): ReactElement {
  // open animation, grows outwardly
  const transformOrigin: string = (() => {
    switch (placement) {
      case "bottom":
        return "top center";
      case "bottom-start":
        return "top left";
      case "bottom-end":
        return "top right";
      case "top":
        return "bottom center";
      case "top-start":
        return "bottom left";
      case "top-end":
        return "bottom right";
      case "right":
        return "center left";
      case "right-start":
        return "top left";
      case "right-end":
        return "bottom left";
      case "left":
        return "center right";
      case "left-start":
        return "top right";
      case "left-end":
        return "bottom right";
    }
  })();

  // fade in
  useEffect(() => {
    const dropdown = ref.current;
    if (!dropdown) return;
    dropdown.style.display = "block";
    dropdown.style.transition = "opacity 0.2s ease, transform 0.2s ease";
    requestAnimationFrame(() => {
      dropdown.style.opacity = "1";
      dropdown.style.transform = "scale(1)";
    });
  }, [ref]);

  // close on click outside
  useEffect(() => {
    const handleClick = (e: MouseEvent | TouchEvent) => {
      if (
        ref.current &&
        !ref.current.contains(e.target as Node) &&
        referenceRef.current &&
        !referenceRef.current.contains(e.target as Node)
      ) {
        onClose?.();
      }
    };
    window.addEventListener("pointerdown", handleClick, { passive: true });
    return () => window.removeEventListener("pointerdown", handleClick);
  }, [onClose, ref, referenceRef]);

  return (
    <div
      ref={setRef}
      style={{
        opacity: 0,
        transformOrigin,
        transform: "scale(0.9)",
        zIndex: 50,
        maxWidth,
        maxHeight,
        ...style,
      }}
      className={cn(className)}
    >
      {children}
    </div>
  );
}
