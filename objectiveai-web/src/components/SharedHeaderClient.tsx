"use client";

import { Provider } from "@/provider";
import { ReactElement, useCallback, useState } from "react";
import { Dropdown, Popup } from "./Popup";
import cn from "classnames";
import Image from "next/image";
import Link from "next/link";
import { LiteralUnion, signIn, signOut } from "next-auth/react";
import { BuiltInProviderType } from "next-auth/providers/index";
import { CloseIcon, MoonIcon, SunIcon } from "./Icon";
import { useTheme } from "@/theme";

export function Models({ className }: { className?: string }): ReactElement {
  const [open, setOpen] = useState(false);
  return (
    <Dropdown
      placement="bottom-start"
      open={open}
      onClose={() => setOpen(false)}
      referenceChildren={({ ref }) => (
        <button
          ref={ref}
          className={cn(
            "text-sm",
            "sm:text-base",
            "font-bold",
            open ? "text-highlight-primary" : "text-primary",
            "hover:text-highlight-primary",
            "cursor-pointer",
            className
          )}
          onClick={() => setOpen((prev) => !prev)}
        >
          Models
        </button>
      )}
      dropdownChildren={
        <div
          className={cn(
            "border",
            "border-muted-secondary",
            "rounded-lg",
            "translate-y-1",
            "bg-background-primary"
          )}
        >
          <ul className={cn("space-y-1", "my-1.5", "mx-2")}>
            <li>
              <Link
                href="/score/models"
                className={cn(
                  "inline-block",
                  "text-sm",
                  "sm:text-base",
                  "font-bold",
                  "text-primary",
                  "hover:text-highlight-primary"
                )}
              >
                Score Models
              </Link>
            </li>
            <li>
              <Link
                href="/score/llms"
                className={cn(
                  "inline-block",
                  "text-sm",
                  "sm:text-base",
                  "font-bold",
                  "text-primary",
                  "hover:text-highlight-primary"
                )}
              >
                Score LLMs
              </Link>
            </li>
          </ul>
        </div>
      }
    />
  );
}

export function Legal({ className }: { className?: string }): ReactElement {
  const [open, setOpen] = useState(false);
  return (
    <Dropdown
      placement="bottom-start"
      open={open}
      onClose={() => setOpen(false)}
      referenceChildren={({ ref }) => (
        <button
          ref={ref}
          className={cn(
            "text-sm",
            "sm:text-base",
            "font-bold",
            open ? "text-highlight-primary" : "text-primary",
            "hover:text-highlight-primary",
            "cursor-pointer",
            className
          )}
          onClick={() => setOpen((prev) => !prev)}
        >
          Legal
        </button>
      )}
      dropdownChildren={
        <div
          className={cn(
            "border",
            "border-muted-secondary",
            "rounded-lg",
            "translate-y-1",
            "bg-background-primary"
          )}
        >
          <ul className={cn("space-y-1", "my-1.5", "mx-2")}>
            <li>
              <Link
                href="/terms"
                className={cn(
                  "inline-block",
                  "text-sm",
                  "sm:text-base",
                  "font-bold",
                  "text-primary",
                  "hover:text-highlight-primary"
                )}
              >
                Terms
              </Link>
            </li>
            <li>
              <Link
                href="/privacy"
                className={cn(
                  "inline-block",
                  "text-sm",
                  "sm:text-base",
                  "font-bold",
                  "text-primary",
                  "hover:text-highlight-primary"
                )}
              >
                Privacy
              </Link>
            </li>
          </ul>
        </div>
      }
    />
  );
}

export function UserProfile({
  session,
  className,
}: {
  session?: Provider.TokenSession;
  className?: string;
}): ReactElement {
  const [open, setOpen] = useState(false);
  const [signingIn, setSigningIn] = useState(false);

  // handle click
  const handleClick = useCallback(() => {
    if (session === undefined || session.session === undefined) {
      setSigningIn((prev) => !prev);
    } else {
      setOpen((prev) => !prev);
    }
  }, [session]);

  return (
    <>
      <Dropdown
        placement="bottom-end"
        open={open}
        onClose={() => setOpen(false)}
        referenceChildren={({ ref }) => (
          <button
            ref={ref}
            className={cn(
              "flex",
              "border",
              "rounded-xl",
              "h-5",
              "sm:h-6",
              open
                ? "border-highlight-primary"
                : cn("border-secondary", "hover:border-highlight-secondary"),
              "cursor-pointer",
              className
            )}
            onClick={handleClick}
          >
            <div
              className={cn(
                "relative",
                "w-5",
                "h-5",
                "sm:w-6",
                "sm:h-6",
                "self-center"
              )}
            >
              <Image
                src={session?.session?.user?.image ?? "/clerk.png"}
                alt="."
                className={cn(
                  "-translate-x-px",
                  "rounded-full",
                  "object-cover"
                )}
                fill
                onError={(e) => (e.currentTarget.src = "/clerk.png")}
              />
            </div>
            <span
              className={cn(
                "pl-1",
                "pr-2",
                "font-medium",
                "text-primary",
                "text-xs",
                "sm:text-base",
                "self-center"
              )}
            >
              {session === undefined || session.session === undefined
                ? "Sign In"
                : session.session.user?.name ?? "Unknown User"}
            </span>
          </button>
        )}
        dropdownChildren={
          <div
            className={cn(
              "border",
              "border-muted-secondary",
              "rounded-lg",
              "translate-y-1",
              "-translate-x-1",
              "bg-background-primary"
            )}
          >
            <ul className={cn("space-y-1", "my-1.5", "mx-2")}>
              <li className={cn("m-1")}>
                <Link
                  href="/account/credits"
                  className={cn(
                    "inline-block",
                    "text-sm",
                    "sm:text-base",
                    "font-bold",
                    "text-primary",
                    "hover:text-highlight-primary"
                  )}
                >
                  Credits
                </Link>
              </li>
              <li className={cn("m-1")}>
                <Link
                  href="/account/keys"
                  className={cn(
                    "inline-block",
                    "text-sm",
                    "sm:text-base",
                    "font-bold",
                    "text-primary",
                    "hover:text-highlight-primary"
                  )}
                >
                  Keys
                </Link>
              </li>
              <li className={cn("m-1")}>
                <button
                  className={cn(
                    "inline-block",
                    "text-sm",
                    "sm:text-base",
                    "font-bold",
                    "text-primary",
                    "hover:text-highlight-primary",
                    "cursor-pointer"
                  )}
                  onClick={() => signOut()}
                >
                  Sign Out
                </button>
              </li>
            </ul>
          </div>
        }
      />
      {signingIn && <SignInPopup onClose={() => setSigningIn(false)} />}
    </>
  );
}

function SignInPopup({
  onClose,
  className,
}: {
  onClose?: () => void;
  className?: string;
}): ReactElement {
  return (
    <Popup<HTMLDivElement> className={cn(className)}>
      {(ref) => (
        <div
          ref={ref}
          className={cn(
            "border",
            "border-muted-secondary",
            "pb-8",
            "rounded-lg",
            "bg-background-primary"
          )}
        >
          <div className={cn("flex", "justify-end", "py-2", "pr-2")}>
            <button
              onClick={onClose}
              className={cn(
                "text-secondary",
                "hover:text-primary",
                "cursor-pointer"
              )}
            >
              <CloseIcon className={cn("h-4")} />
            </button>
          </div>
          <div
            className={cn(
              "w-96",
              "max-w-[calc(100dvw-var(--spacing)*4)]",
              "px-8",
              "space-y-4"
            )}
          >
            <SignInProvider
              src="/google.svg"
              title="Google"
              provider="google"
              className={cn("bg-white", "text-black")}
            />
            <SignInProvider
              src="/github.svg"
              title="GitHub"
              provider="github"
              className={cn("bg-[#0D1117]", "text-white")}
            />
            <SignInProvider
              src="/twitter.svg"
              title="X"
              provider="twitter"
              className={cn("bg-black", "text-white")}
            />
            <SignInProvider
              src="/reddit.svg"
              title="Reddit"
              provider="reddit"
              className={cn("bg-[#FF4500]", "text-white")}
            />
          </div>
        </div>
      )}
    </Popup>
  );
}

function SignInProvider({
  src,
  title,
  provider,
  className,
}: {
  src: string;
  title: string;
  provider: LiteralUnion<BuiltInProviderType>;
  className?: string;
}): ReactElement {
  return (
    <button
      className={cn(
        "w-full",
        "flex",
        "border-2",
        "border-muted-secondary",
        "hover:border-primary",
        "rounded-lg",
        "py-4",
        "px-8",
        "cursor-pointer",
        "gap-3",
        className
      )}
      onClick={() => signIn(provider)}
    >
      <Image src={src} alt={title} width={24} height={24} className={cn()} />
      <span className={cn("text-lg")}>Sign in with {title}</span>
    </button>
  );
}

export function ThemeSelector({
  className,
}: {
  className?: string;
}): ReactElement {
  const { theme, setTheme } = useTheme();
  return (
    <button
      className={cn(
        "flex",
        "h-5",
        "sm:h-6",
        "px-1",
        "border",
        "rounded-xl",
        "border-secondary",
        "items-center",
        theme === undefined ? "cursor-not-allowed" : "cursor-pointer",
        "hover:border-highlight-primary",
        className
      )}
      disabled={theme === undefined}
      onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
    >
      <SunIcon
        className={cn(
          "h-4",
          "sm:h-5",
          theme === "light" ? "text-primary" : "text-muted-secondary"
        )}
      />
      <MoonIcon
        className={cn(
          "h-4",
          "sm:h-5",
          theme === "dark" ? "text-primary" : "text-muted-secondary",
          "ml-1"
        )}
      />
    </button>
  );
}
