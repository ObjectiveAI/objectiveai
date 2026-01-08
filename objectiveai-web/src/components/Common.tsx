"use client";

import {
  ReactElement,
  ReactNode,
  RefObject,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import cn from "classnames";
import {
  CheckIcon,
  DragMove,
  ExpandUpDown,
  OpenSectionIcon,
  ResetLeft,
  SmallCloseIcon,
} from "@/components/Icon";
import { Dropdown, Popup } from "@/components/Popup";
import { Provider } from "@/provider";
import OpenAI from "openai";
import {
  DraggableAttributes,
  DndContext,
  PointerSensor,
  useSensor,
  useSensors,
  closestCenter,
} from "@dnd-kit/core";
import { SyntheticListenerMap } from "@dnd-kit/core/dist/hooks/utilities";
import {
  SortableContext,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { restrictToVerticalAxis } from "@dnd-kit/modifiers";
import { createPortal } from "react-dom";

export const BgSecondary = "bg-[rgb(18,4,40)]";
export const BgUser = "bg-[rgb(28,26,38)]";
export const BgAssistant = BgUser;

export function SelectorPopup<T extends HTMLElement>({
  childrenSelected,
  children,
  title,
  onReset,
  expanderLocation,
  disabled,
  isDefault,
  loading,
  className,
  noTextSize,
}: {
  childrenSelected?: ReactNode;
  children?: {
    (props: { ref: RefObject<T | null>; onClose: () => void }): ReactNode;
  };
  title?: ReactNode;
  onReset?: () => void;
  expanderLocation: "left" | "right";
  disabled?: boolean;
  isDefault?: boolean;
  loading?: boolean;
  className?: string;
  noTextSize?: boolean;
}): ReactElement {
  const [open, setOpen] = useState<boolean>(false);
  return (
    <>
      <ConfigContainer
        title={title}
        onReset={onReset}
        canReset={!disabled && !isDefault}
        className={cn(className)}
      >
        <button
          className={cn(
            "flex",
            "items-center",
            disabled ? "px-2" : expanderLocation === "left" ? "pr-2" : "pl-2",
            disabled
              ? "cursor-not-allowed"
              : loading
              ? "cursor-wait"
              : "cursor-pointer",
            "p-1",
            "rounded-lg",
            isDefault && "text-muted-primary",
            "select-none",
            open
              ? cn("bg-highlight-secondary", "text-primary")
              : cn("hover:bg-highlight-muted", "hover:text-primary"),
            "min-w-0"
          )}
          disabled={disabled}
          onClick={() => !disabled && !loading && setOpen(!open)}
        >
          {expanderLocation === "left" && !disabled && (
            <ExpandUpDown className={cn("h-4", "min-w-4")} />
          )}
          <span
            className={cn(
              "whitespace-nowrap",
              "overflow-hidden",
              "overflow-ellipsis",
              !noTextSize && cn("text-sm", "sm:text-base")
            )}
          >
            {childrenSelected}
          </span>
          {expanderLocation === "right" && !disabled && (
            <ExpandUpDown className={cn("h-4", "min-w-4")} />
          )}
        </button>
      </ConfigContainer>
      {open &&
        createPortal(
          <Popup<T>>
            {(ref) => children?.({ ref, onClose: () => setOpen(false) })}
          </Popup>,
          document.body
        )}
    </>
  );
}

export function SelectorDropdown<T>({
  items,
  itemsEnabled,
  onChange,
  onReset,
  popupClassName,
  className,
  itemName,
  children,
  childrenSelected = children,
  selected,
  disabled = false,
  isDefault = false,
  title,
  loading = false,
  placement,
  expanderLocation,
  noTextSize,
}: {
  items: T[];
  itemsEnabled?: boolean[];
  onChange?: (item: T) => void;
  onReset?: () => void;
  popupClassName?: string;
  className?: string;
  itemName: (item: T) => string;
  children: (item: T) => ReactNode;
  childrenSelected?: (item: T) => ReactNode;
  selected: T;
  disabled?: boolean;
  isDefault?: boolean;
  title?: ReactNode;
  loading?: boolean;
  placement:
    | "bottom"
    | "bottom-start"
    | "bottom-end"
    | "top"
    | "top-start"
    | "top-end";
  expanderLocation: "left" | "right";
  noTextSize?: boolean;
}): ReactElement {
  const [open, setOpen] = useState<boolean>(false);
  const handleChange = (item: T) => {
    setOpen(false);
    onChange?.(item);
  };
  return (
    <ConfigContainer
      title={title}
      onReset={onReset}
      canReset={!disabled && !isDefault}
      className={cn(className)}
    >
      <Dropdown
        placement={placement}
        open={open}
        onClose={() => setOpen(false)}
        referenceChildren={({ ref }) => (
          <button
            ref={ref}
            className={cn(
              "flex",
              "items-center",
              disabled ? "px-2" : expanderLocation === "left" ? "pr-2" : "pl-2",
              disabled
                ? "cursor-not-allowed"
                : loading
                ? "cursor-wait"
                : "cursor-pointer",
              "p-1",
              "rounded-lg",
              isDefault && "text-muted-primary",
              "select-none",
              open
                ? cn("bg-highlight-secondary", "text-primary")
                : cn("hover:bg-highlight-muted", "hover:text-primary"),
              "min-w-0"
            )}
            disabled={disabled}
            onClick={() => !disabled && !loading && setOpen(!open)}
          >
            {expanderLocation === "left" && !disabled && (
              <ExpandUpDown className={cn("h-4", "min-w-4")} />
            )}
            <span
              className={cn(
                "whitespace-nowrap",
                "overflow-hidden",
                "overflow-ellipsis",
                !noTextSize && cn("text-sm", "sm:text-base")
              )}
            >
              {childrenSelected(selected)}
            </span>
            {expanderLocation === "right" && !disabled && (
              <ExpandUpDown className={cn("h-4", "min-w-4")} />
            )}
          </button>
        )}
        dropdownChildren={
          <SelectorDropdownContent<T>
            placement={placement}
            items={items}
            itemsEnabled={itemsEnabled}
            selected={selected}
            onChange={handleChange}
            itemKey={(item, index) => `${itemName(item)}-${index}`}
            className={cn(popupClassName)}
          >
            {children}
          </SelectorDropdownContent>
        }
      />
    </ConfigContainer>
  );
}

export function SelectorDropdownContent<T>({
  placement,
  items,
  itemsEnabled,
  selected,
  onChange,
  children,
  itemKey,
  className,
}: {
  // the placement of this dropdown
  placement:
    | "bottom"
    | "bottom-start"
    | "bottom-end"
    | "top"
    | "top-start"
    | "top-end";
  items: T[];
  itemsEnabled?: boolean[];
  selected?: T;
  onChange: (value: T) => void;
  children: (item: T) => ReactNode;
  itemKey: (item: T, index: number) => React.Key;
  className?: string;
}) {
  // scroll to current item
  const selectorRef = useRef<HTMLDivElement>(null);
  const selectedRef = useRef<HTMLLIElement>(null);
  useEffect(() => {
    if (selectedRef.current && selectorRef.current) {
      selectorRef.current.scrollTo({
        top:
          selectedRef.current.offsetTop +
          selectedRef.current.offsetHeight / 2 -
          selectorRef.current.clientHeight / 2,

        // behavior: "smooth",
      });
    }
  }, [selectedRef, selectorRef]);

  const isEnabled = useCallback(
    (index: number): boolean => {
      if (itemsEnabled === undefined) {
        return true;
      } else {
        return itemsEnabled[index];
      }
    },
    [itemsEnabled]
  );

  return (
    <div
      ref={selectorRef}
      className={cn(
        "[direction:ltr]",
        "border",
        "border-muted-secondary",
        "rounded-lg",
        "overflow-y-auto",
        "[overflow-wrap:anywhere]",
        placement.startsWith("bottom") ? "translate-y-1" : "-translate-y-1",
        "bg-background-primary",
        className
      )}
    >
      <ul>
        {items.map((item, index) => (
          <li
            key={itemKey(item, index)}
            ref={item === selected ? selectedRef : null}
            className={cn("m-1")}
          >
            <button
              className={cn(
                "w-full",
                "text-left",
                isEnabled(index)
                  ? cn("cursor-pointer")
                  : cn("text-muted-primary", "cursor-not-allowed"),
                "select-none",
                "py-0.5",
                "px-4",
                "rounded-lg",
                item === selected
                  ? "bg-highlight-secondary"
                  : cn("hover:bg-highlight-muted", "focus:bg-highlight-muted")
              )}
              onClick={() => onChange(item)}
              disabled={!isEnabled(index)}
            >
              {children(item)}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

export function NumberSlider({
  ref,
  title,
  onChange,
  value,
  minValue,
  maxValue,
  defaultValue,
  precision,
  step,
  disabled,
  className,
}: {
  ref?: RefObject<HTMLDivElement | null>;
  title?: ReactNode;
  onChange: (value: number) => void;
  value: number;
  minValue: number;
  maxValue: number;
  defaultValue: number;
  precision: number;
  step?: number;
  disabled?: boolean;
  className?: string;
}): ReactElement {
  const [localValue, setLocalValue] = useState<number>(value);
  const [grabbing, setGrabbing] = useState<boolean>(false);
  useEffect(() => setLocalValue(value), [value]);

  const handleReset = () => {
    setLocalValue(defaultValue);
    onChange(defaultValue);
  };

  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = parseFloat(e.target.value);
    setLocalValue(newValue);
  };

  const handleSliderSubmit = () => {
    onChange(localValue);
    setGrabbing(false);
  };

  const handleInputSubmit = (value: number) => {
    setLocalValue(value);
    onChange(value);
  };

  const handleSliderStart = () => {
    setGrabbing(true);
  };

  return (
    <div className={cn("flex", "flex-col", className)} ref={ref}>
      <ConfigContainer
        title={title}
        onReset={handleReset}
        canReset={
          !disabled &&
          value !== defaultValue &&
          defaultValue >= minValue &&
          defaultValue <= maxValue
        }
      >
        <NumberInput
          onSubmit={handleInputSubmit}
          value={localValue}
          minValue={minValue}
          maxValue={maxValue}
          defaultValue={defaultValue}
          precision={precision}
          disabled={disabled}
        />
      </ConfigContainer>
      <input
        type="range"
        min={minValue}
        max={maxValue}
        step={step ?? 1}
        value={localValue}
        onChange={handleSliderChange}
        onPointerUp={handleSliderSubmit}
        onBlur={handleSliderSubmit}
        onKeyUp={handleSliderSubmit}
        onPointerCancel={handleSliderSubmit}
        onPointerDown={handleSliderStart}
        disabled={disabled}
        className={cn(
          "w-full",
          disabled
            ? "cursor-not-allowed"
            : grabbing
            ? "cursor-grabbing"
            : "cursor-grab",
          localValue === defaultValue
            ? cn("hover:accent-highlight-secondary", "accent-highlight-muted")
            : "accent-highlight-secondary"
        )}
      />
    </div>
  );
}

export function BooleanSelector({
  ref,
  title,
  onChange,
  value,
  disabled,
  className,
}: {
  ref?: RefObject<HTMLDivElement | null>;
  title?: ReactNode;
  onChange: (value: boolean) => void;
  value: boolean;
  disabled?: boolean;
  className?: string;
}): ReactElement {
  const handleChange = () => {
    onChange(!value);
  };
  return (
    <ConfigContainer ref={ref} title={title} className={cn(className)}>
      <IconButton
        onClick={handleChange}
        disabled={disabled}
        className={cn("py-px")}
        size="md"
      >
        {({ className }) =>
          value ? (
            <CheckIcon className={cn(className)} />
          ) : (
            <SmallCloseIcon className={cn(className)} />
          )
        }
      </IconButton>
    </ConfigContainer>
  );
}

export function NumberInput({
  onSubmit,
  value,
  minValue,
  maxValue,
  defaultValue,
  precision,
  disabled,
  className,
}: {
  onSubmit: (value: number) => void;
  value: number;
  minValue: number;
  maxValue: number;
  defaultValue: number;
  precision: number;
  disabled?: boolean;
  className?: string;
}): ReactElement {
  const valueStr = value.toFixed(precision);
  const defaultValueStr = defaultValue.toFixed(precision);
  const [localValue, setLocalValue] = useState<string>(valueStr);
  const [typing, setTyping] = useState<boolean>(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const ignoreBlur = useRef<boolean>(false);

  useEffect(() => {
    setLocalValue(valueStr);
  }, [valueStr]);

  const handleSubmit = () => {
    if (localValue !== valueStr) {
      let newValueStr = "";
      let hasDecimal = false;
      let hasNegative = false;
      let hasNumber = false;
      for (const char of localValue) {
        if (char === ".") {
          if (hasDecimal) {
            break;
          } else {
            hasDecimal = true;
          }
        } else if (char === "-") {
          if (hasNegative) {
            break;
          } else {
            hasNegative = true;
          }
        } else {
          hasNumber = true;
        }
        newValueStr += char;
      }
      let newValue: number | null;
      if (!hasNumber) {
        newValue = null;
      } else if (hasDecimal) {
        const [int, dec] = newValueStr.split(".");
        if (dec.length === 0) {
          newValue = parseInt(int, 10);
        } else {
          newValue = parseFloat(`${int}.${dec.slice(0, precision)}`);
        }
      } else {
        newValue = parseInt(newValueStr, 10);
      }
      if (newValue === null || newValue === value) {
        setLocalValue(valueStr);
      } else if (newValue < minValue) {
        onSubmit(minValue);
        setLocalValue(minValue.toFixed(precision));
      } else if (newValue > maxValue) {
        onSubmit(maxValue);
        setLocalValue(maxValue.toFixed(precision));
      } else {
        onSubmit(newValue);
        setLocalValue(newValue.toFixed(precision));
      }
    }
    setTyping(false);
  };
  const handleCancel = () => {
    setLocalValue(valueStr);
    setTyping(false);
  };
  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    let newLocalValue = "";
    for (const char of e.target.value) {
      if (
        (char === "." && precision > 0) ||
        (char >= "0" && char <= "9") ||
        (char === "-" && minValue < 0)
      ) {
        newLocalValue += char;
      }
    }
    setLocalValue(newLocalValue);
    setTyping(true);
  };
  return (
    <div
      className={cn(
        disabled && "cursor-not-allowed",
        "px-1",
        "border",
        localValue === defaultValueStr
          ? cn("border-muted-secondary")
          : cn("border-white"),
        "rounded-lg",
        className
      )}
    >
      <input
        ref={inputRef}
        type="text"
        inputMode="numeric"
        value={localValue === defaultValueStr && !typing ? "" : localValue}
        placeholder={defaultValueStr}
        onChange={handleChange}
        disabled={disabled}
        onBlur={() => {
          if (ignoreBlur.current) {
            ignoreBlur.current = false;
          } else {
            handleSubmit();
          }
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            handleSubmit();
            ignoreBlur.current = true;
            inputRef.current?.blur();
          } else if (e.key === "Escape") {
            handleCancel();
            ignoreBlur.current = true;
            inputRef.current?.blur();
          }
        }}
        className={cn(
          disabled && "cursor-not-allowed",
          "placeholder-muted-primary",
          "outline-none",
          "text-right",
          "font-mono"
          // "text-sm",
          // "sm:text-base"
        )}
        style={{
          width: `${localValue.length}ch`,
        }}
      />
    </div>
  );
}

export function ConfigContainer({
  ref,
  title,
  children,
  className,
  onReset,
  canReset = false,
}: {
  ref?: RefObject<HTMLDivElement | null>;
  title?: ReactNode;
  children?: ReactNode;
  className?: string;
  onReset?: () => void;
  canReset?: boolean;
}): ReactElement {
  if (title !== undefined) {
    return (
      <div
        ref={ref}
        className={cn(
          "flex",
          title !== undefined && cn("w-full", "justify-between"),
          "items-center",
          "min-w-0",
          className
        )}
      >
        <h3
          className={cn(
            "whitespace-nowrap",
            "mr-2",
            "text-primary",
            "text-sm",
            "sm:text-base"
          )}
        >
          {title}
        </h3>
        <div className={cn("flex", "items-center", "min-w-0")}>
          {canReset && onReset && (
            <IconButton onClick={onReset} className={cn("mr-1")}>
              {({ className }) => <ResetLeft className={className} />}
            </IconButton>
          )}
          {children}
        </div>
      </div>
    );
  } else {
    return (
      <div
        ref={ref}
        className={cn("flex", "items-center", "min-w-0", className)}
      >
        {canReset && onReset && (
          <IconButton onClick={onReset} className={cn("mr-1")}>
            {({ className }) => <ResetLeft className={className} />}
          </IconButton>
        )}
        {children}
      </div>
    );
  }
}

export function IconButton({
  ref,
  children,
  disabled = false,
  onClick,
  open,
  className,
  size = "sm",
}: {
  ref?: RefObject<HTMLButtonElement | null>;
  children: (props: { className: string }) => ReactNode;
  disabled?: boolean;
  onClick?: () => void;
  open?: boolean;
  className?: string;
  size?: "sm" | "md" | "lg";
}): ReactElement {
  return (
    <button
      ref={ref}
      className={cn(
        open === true
          ? cn("bg-highlight-secondary", "text-primary")
          : cn("text-muted-primary", "hover:text-primary"),
        disabled
          ? cn("cursor-not-allowed", "hover:bg-highlight-muted")
          : cn(
              "cursor-pointer",
              open === false
                ? "hover:bg-highlight-muted"
                : "hover:bg-highlight-secondary"
            ),
        "px-2",
        size === "sm" && "py-1",
        size === "lg" && cn("py-0.5", "sm:py-1"),
        "rounded-lg",
        className
      )}
      onClick={onClick}
      disabled={disabled}
    >
      {children({
        className: cn(
          size === "sm" && "h-4",
          size === "md" && "h-6",
          size === "lg" && "h-6"
        ),
      })}
    </button>
  );
}

export function SliderIconButton({
  children,
  disabled = false,
  onClick,
  className,
  size = "md",
}: {
  children: (props: { className: string }) => ReactNode;
  disabled?: boolean;
  onClick?: () => void;
  className?: string;
  size?: "sm" | "md";
}): ReactElement {
  const Divider = () => (
    <span className={cn("border-muted-secondary", "border-b", "flex-grow")} />
  );
  return (
    <div className={cn("flex", "items-end", className)}>
      <Divider />
      <IconButton
        disabled={disabled}
        onClick={disabled ? undefined : onClick}
        size={size}
      >
        {children}
      </IconButton>
      <Divider />
    </div>
  );
}

export function confidenceColor(confidence: number, alpha: number) {
  return `hsla(${
    confidence * 120
  },var(--confidence-saturation),var(--confidence-lightness),${alpha})`;
}

export function Confidence({
  confidence,
  small,
  className,
}: {
  confidence: number;
  small?: boolean;
  className?: string;
}): ReactElement {
  const [confidenceStr, color] = useMemo(() => {
    return [
      new Intl.NumberFormat(undefined, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      }).format(confidence * 100),
      confidenceColor(confidence, 1),
    ];
  }, [confidence]);
  return (
    <>
      <span
        className={cn(
          small ? "text-sm" : "text-base",
          "leading-none",
          "font-mono",
          "font-semibold",
          className
        )}
        style={{ color }}
      >
        {confidenceStr}
      </span>
      <span
        className={cn(
          "text-xs",
          "leading-none",
          "ml-0.5",
          "font-medium",
          className
        )}
        style={{ color }}
      >
        %
      </span>
    </>
  );
}

export function Metadata({
  ref,
  confidence,
  confidenceIsFinal = false,
  cost,
  className,
}: {
  ref?: RefObject<HTMLDivElement | null>;
  confidence?: number;
  confidenceIsFinal?: boolean;
  cost?: number;
  fixedHeight?: boolean;
  className?: string;
}): ReactElement {
  const costFormatted = useMemo(
    () =>
      cost !== undefined
        ? new Intl.NumberFormat(undefined, {
            style: "currency",
            currency: "USD",
            maximumFractionDigits: 6,
          }).format(cost)
        : undefined,
    [cost]
  );
  return (
    <div
      className={cn(
        "flex",
        "gap-x-2",
        "leading-none",
        "items-end",
        "overflow-hidden",
        className
      )}
      ref={ref}
    >
      {confidence !== undefined && (
        <>
          <span className={cn("leading-none")}>
            {!confidenceIsFinal && (
              <span
                className={cn("text-xs", "leading-none", "text-muted-primary")}
              >
                ~
              </span>
            )}
            <Confidence confidence={confidence} />
          </span>
          <span className={cn("text-xs", "leading-none", "text-muted-primary")}>
            confidence
          </span>
        </>
      )}
      {cost !== undefined && (
        <span
          className={cn(
            "text-sm",
            "leading-none",
            "font-mono",
            "text-secondary",
            "ml-auto"
          )}
        >
          {costFormatted}
        </span>
      )}
    </div>
  );
}

export function EmptyText({
  content,
  className,
}: {
  content: string;
  className?: string;
}): ReactElement {
  return (
    <span className={cn("text-muted-primary", "text-sm", "italic", className)}>
      {content}
    </span>
  );
}

export async function openAi(session?: Provider.TokenSession): Promise<OpenAI> {
  const authorization = session
    ? await Provider.TokenSession.authorization(session)
    : "none";
  return new OpenAI({
    apiKey: authorization ?? "none",
    dangerouslyAllowBrowser: true,
    baseURL: process.env.NEXT_PUBLIC_API_URL,
    maxRetries: 0,
  });
}

export function DragButton({
  dragAttributes,
  dragListeners,
  mode,
  onClick,
}: {
  dragAttributes: DraggableAttributes;
  dragListeners: SyntheticListenerMap | undefined;
  mode: "dragging" | "disabled" | "drag" | "click";
  onClick?: () => void;
}): ReactElement {
  return (
    <button
      className={cn(
        mode === "dragging" && cn("cursor-grabbing", "bg-highlight-secondary"),
        mode === "disabled" &&
          cn(
            "text-muted-primary",
            "cursor-not-allowed",
            "hover:bg-highlight-muted"
          ),
        mode === "drag" &&
          cn(
            "text-muted-primary",
            "cursor-grab",
            "hover:bg-highlight-secondary"
          ),
        mode === "click" &&
          cn(
            "text-muted-primary",
            "cursor-pointer",
            "hover:bg-highlight-secondary"
          ),
        "hover:text-primary",
        "px-2",
        "py-1",
        "rounded-lg"
      )}
      {...(mode === "dragging" || mode === "drag" ? dragAttributes : {})}
      {...(mode === "dragging" || mode === "drag" ? dragListeners : {})}
      disabled={mode === "disabled"}
      onClick={mode === "click" ? onClick : undefined}
    >
      <DragMove className={cn("h-4")} />
    </button>
  );
}

export function DragContext({
  items,
  onMove,
  children,
}: {
  items: { id: number }[];
  onMove: (indexFrom: number, indexTo: number) => void;
  children?: ReactNode;
}): ReactElement {
  const dragSensors = useSensors(useSensor(PointerSensor));
  return (
    <DndContext
      sensors={dragSensors}
      collisionDetection={closestCenter}
      modifiers={[restrictToVerticalAxis]}
      onDragEnd={(event) => {
        const { active, over } = event;
        if (over?.id !== undefined && active.id !== over.id) {
          let oldIndex: number | undefined = undefined;
          let newIndex: number | undefined = undefined;
          for (let i = 0; i < items.length; i++) {
            const { id } = items[i];
            if (id === active.id) oldIndex = i;
            if (id === over.id) newIndex = i;
            if (oldIndex !== undefined && newIndex !== undefined) break;
          }
          if (oldIndex !== undefined && newIndex !== undefined) {
            onMove(oldIndex, newIndex);
          }
        }
      }}
    >
      <SortableContext items={items} strategy={verticalListSortingStrategy}>
        {children}
      </SortableContext>
    </DndContext>
  );
}

export function useHasVerticalScrollbar(
  ref: RefObject<HTMLElement | null>
): boolean {
  const [hasScrollbar, setHasScrollbar] = useState<boolean>(false);
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const update = () =>
      setHasScrollbar(element.scrollHeight > element.clientHeight);
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
  return hasScrollbar;
}

export function SectionButton({
  children,
  open,
  onClick,
  disabled,
  loading,
  className,
}: {
  children?: ReactNode;
  open: boolean;
  onClick?: () => void;
  disabled?: boolean;
  loading?: boolean;
  className?: string;
}): ReactElement {
  return (
    <button
      className={cn(
        "underline",
        "text-sm",
        "text-secondary",
        "hover:text-primary",
        loading
          ? "cursor-wait"
          : disabled
          ? "cursor-not-allowed"
          : "cursor-pointer",
        "flex",
        "items-center",
        "gap-1",
        className
      )}
      onClick={onClick}
      disabled={disabled}
    >
      <OpenSectionIcon
        className={cn(
          "h-3",
          "transition-transform",
          "duration-200",
          open ? "rotate-90" : null
        )}
      />
      {children}
    </button>
  );
}

export function SectionButtonServer({
  children,
  childrenContent,
  disabled,
  loading,
  className,
  classNameContent,
  defaultOpen = false,
}: {
  children?: ReactNode;
  childrenContent?: ReactNode;
  disabled?: boolean;
  loading?: boolean;
  className?: string;
  classNameContent?: string;
  defaultOpen?: boolean;
}): ReactElement {
  const [open, setOpen] = useState<boolean>(defaultOpen);
  return (
    <>
      <SectionButton
        open={open}
        onClick={() => setOpen((prev) => !prev)}
        disabled={disabled}
        loading={loading}
        className={className}
      >
        {children}
      </SectionButton>
      <div className={cn(!open && "hidden", classNameContent)}>
        {childrenContent}
      </div>
    </>
  );
}

export function price(value: number, unit: string): string {
  return `$${value.toFixed(6).replace(/\.?0+$/, "")}/${unit}`;
}
