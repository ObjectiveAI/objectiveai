"use client";

import { Provider } from "@/provider";
import { Auth } from "objectiveai";
import {
  Fragment,
  ReactElement,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import cn from "classnames";
import { openAi } from "@/components/Common";
import { SharedFooter } from "../../SharedFooter";
import { SharedHeader } from "../../SharedHeader";
import { Header } from "./Header";
import { CheckIcon, CopyIcon } from "../../Icon";
import { LoadingDots } from "@/components/Loading";
import { costFormatter } from "@/format";

interface KeyWithCost
  extends Omit<Auth.ApiKeyWithCost, "created" | "expires" | "disabled"> {
  created: Date;
  expires: Date | null;
  disabled: Date | null;
}

function convertKey(key: Auth.ApiKeyWithCost): KeyWithCost {
  const { created, expires, disabled, ...rest } = key;
  return {
    ...rest,
    created: new Date(created),
    expires: expires ? new Date(expires) : null,
    disabled: disabled ? new Date(disabled) : null,
  };
}

export function Keys({
  session,
  className,
}: {
  session: Provider.TokenSession;
  className?: string;
}): ReactElement {
  const now = useMemo(() => Date.now(), []);
  const [sort, setSort] = useState<"created" | "cost">("created");
  const [keys, setKeys] = useState<KeyWithCost[]>();

  // split keys into enabled and disabled, then sort them
  const keysSorted = useMemo(() => {
    if (!keys) return [];
    const enabledKeys = [];
    const disabledKeys = [];
    for (const key of keys) {
      if (
        (key.disabled && key.disabled.getTime() < now) ||
        (key.expires && key.expires.getTime() < now)
      ) {
        disabledKeys.push(key);
      } else {
        enabledKeys.push(key);
      }
    }
    const sortKeys = (keys: KeyWithCost[]): KeyWithCost[] =>
      keys.sort((a, b) => {
        if (sort === "created") {
          return new Date(a.created).getTime() - new Date(b.created).getTime();
        } else {
          return a.cost - b.cost;
        }
      });
    return [...sortKeys(enabledKeys), ...sortKeys(disabledKeys)];
  }, [keys, sort, now]);

  // load keys exactly once
  const hookExecuted = useRef(false);
  useEffect(() => {
    if (hookExecuted.current) return;
    hookExecuted.current = true;
    (async () => {
      const openai = await openAi(session);
      const { data } = await Auth.ApiKey.list(openai);
      setKeys(data.map(convertKey));
    })();
  }, [session]);

  // add key to existing keys
  const handleAddKey = (key: Auth.ApiKey) => {
    setKeys((prev) => [...(prev || []), convertKey({ ...key, cost: 0 })]);
  };

  // disable key
  const handleDisableKey = (key: Auth.ApiKey) =>
    setKeys((prev) =>
      prev?.map((k) =>
        k.api_key === key.api_key
          ? { ...k, disabled: new Date(key.disabled ?? Date.now()) }
          : k
      )
    );

  return (
    <main className={cn("h-[100dvh]", "flex", "flex-col", className)}>
      <SharedHeader session={session} />
      <Header
        sort={sort}
        onSortChanged={setSort}
        onAddKey={handleAddKey}
        session={session}
        loading={keys === undefined}
        now={now}
      />
      <div className={cn("flex-grow", "overflow-y-auto")}>
        <div
          className={cn(
            "w-[calc(var(--spacing)*192)]",
            "max-w-[calc(100%-var(--spacing)*4)]",
            "whitespace-nowrap",
            "mx-auto"
          )}
        >
          <ul className={cn("px-2")}>
            {keysSorted.map((key, index) => (
              <Fragment key={key.api_key}>
                {index > 0 && (
                  <li
                    className={cn(
                      "h-4",
                      "border-t",
                      "border-muted-secondary",
                      "mt-4"
                    )}
                  />
                )}
                <Key
                  keyWithCost={key}
                  now={now}
                  session={session}
                  onDisable={handleDisableKey}
                  className={cn(index === 0 && "mt-4")}
                />
              </Fragment>
            ))}
          </ul>
        </div>
      </div>
      <SharedFooter />
    </main>
  );
}

function Key({
  keyWithCost: {
    api_key,
    created: createdProp,
    name,
    cost: costProp,
    expires: expiresProp,
    description,
    disabled: disabledProp,
  },
  now,
  session,
  onDisable,
  className,
}: {
  keyWithCost: KeyWithCost;
  now: number;
  session: Provider.TokenSession;
  onDisable: (key: Auth.ApiKey) => void;
  className?: string;
}): ReactElement {
  const [disabling, setDisabling] = useState(false);
  const [copied, setCopied] = useState(false);

  const { created, expiresOrDisabled, isDisabled, isExpired, cost } =
    useMemo(() => {
      return {
        created: createdProp.toLocaleString(),
        expiresOrDisabled: disabledProp
          ? disabledProp.toLocaleString()
          : expiresProp
          ? expiresProp.toLocaleString()
          : undefined,
        isDisabled: disabledProp !== null,
        isExpired: expiresProp !== null && expiresProp.getTime() < now,
        cost: costFormatter().format(costProp),
      };
    }, [createdProp, costProp, expiresProp, disabledProp, now]);

  const handleDisable = () => {
    setDisabling(true);
    (async () => {
      const openai = await openAi(session);
      const key = await Auth.ApiKey.remove(openai, api_key);
      setDisabling(false);
      onDisable(key);
    })();
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(api_key);
    setCopied(true);
    setTimeout(() => setCopied(false), 1000);
  };

  return (
    <li className={cn(className)}>
      <div className={cn("flex", "justify-between", "items-center")}>
        <div
          className={cn(
            "text-lg",
            "max-w-[80dvw]",
            "overflow-hidden",
            "text-ellipsis",
            "align-text-top"
          )}
        >
          <span
            className={cn(
              (isDisabled || isExpired) && cn("line-through", "mr-2")
            )}
          >
            {name}
          </span>
          {isDisabled && (
            <span className={cn("text-xs", "text-secondary")}>(disabled)</span>
          )}
          {isExpired && (
            <span className={cn("text-xs", "text-secondary")}>(expired)</span>
          )}
        </div>
        {!isDisabled && !isExpired && (
          <button
            className={cn(
              "font-bold",
              "text-secondary",
              "hover:text-highlight-primary",
              "transition-colors",
              "duration-200",
              "text-sm",
              disabling ? cn("cursor-wait") : cn("underline", "cursor-pointer")
            )}
            onClick={handleDisable}
            disabled={disabling}
          >
            {disabling ? <LoadingDots /> : "disable"}
          </button>
        )}
      </div>
      <div className={cn("flex", "justify-between", "items-center")}>
        <span className={cn("inline-flex", "items-center")}>
          <span
            className={cn("text-xs", "font-mono", "text-muted-primary", "mr-2")}
          >
            {api_key.slice(0, 6)}...{api_key.slice(-6)}
          </span>
          <button
            className={cn(
              !copied && cn("text-secondary", "hover:text-primary"),
              "transition-colors",
              "duration-200",
              "cursor-pointer"
            )}
            onClick={handleCopy}
          >
            {copied ? (
              <CheckIcon className={cn("h-4")} />
            ) : (
              <CopyIcon className={cn("h-4")} />
            )}
          </button>
        </span>
        <span className={cn("inline-flex", "items-center")}>
          <span className={cn("font-mono", "text-secondary", "mr-2")}>
            {cost}
          </span>
          <span className={cn("text-xs", "text-muted-primary")}>spent</span>
        </span>
      </div>
      <div className={cn("flex", "items-center", "h-6")}>
        <span className={cn("inline-flex", "items-center")}>
          <span className={cn("text-xs", "text-muted-primary", "mr-2")}>
            created
          </span>
          <span className={cn("text-sm", "text-secondary")}>{created}</span>
        </span>
      </div>
      <div className={cn("flex", "items-center", "h-6")}>
        <span className={cn("inline-flex", "items-center")}>
          <span className={cn("text-xs", "text-muted-primary", "mr-2")}>
            {isDisabled ? "disabled" : isExpired ? "expired" : "expires"}
          </span>
          <span className={cn("text-sm", "text-secondary")}>
            {expiresOrDisabled ?? "never"}
          </span>
        </span>
      </div>
      {description && (
        <div
          className={cn(
            "whitespace-normal",
            "[overflow-wrap:anywhere]",
            "text-sm",
            "text-secondary",
            "mt-1"
          )}
        >
          {description}
        </div>
      )}
    </li>
  );
}
