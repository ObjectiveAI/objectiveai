import { ReactElement, Ref } from "react";
import cn from "classnames";
import Link from "next/link";
import {
  DiscordIcon,
  GitHubIcon,
  LinkedInIcon,
  TwitterIcon,
  YouTubeIcon,
} from "./Icon";

export function SharedFooter({
  ref,
  className,
}: {
  ref?: Ref<HTMLDivElement | null>;
  className?: string;
}): ReactElement {
  return (
    <div
      className={cn(
        "flex",
        "gap-x-3",
        "justify-between",
        "items-end",
        "px-1",
        "pb-1",
        "pt-0.5",
        className
      )}
      ref={ref}
    >
      <span className={cn("text-xs", "text-secondary")}>
        Objective Artificial Intelligence, Inc.
      </span>
      <div className={cn("flex", "gap-x-3", "pr-1", className)}>
        <LinkIcon href="https://discord.gg/gbNFHensby">
          {({ className }) => <DiscordIcon className={cn(className)} />}
        </LinkIcon>
        <LinkIcon href="https://github.com/ObjectiveAI">
          {({ className }) => <GitHubIcon className={cn(className)} />}
        </LinkIcon>
        <LinkIcon href="https://www.linkedin.com/company/107735448">
          {({ className }) => <LinkedInIcon className={cn(className)} />}
        </LinkIcon>
        <LinkIcon href="https://x.com/objectv_ai">
          {({ className }) => <TwitterIcon className={cn(className)} />}
        </LinkIcon>
        <LinkIcon href="https://www.youtube.com/@Objective-AI">
          {({ className }) => <YouTubeIcon className={cn(className)} />}
        </LinkIcon>
      </div>
    </div>
  );
}

function LinkIcon({
  href,
  children,
  className,
}: {
  href: string;
  children: (props: { className?: string }) => ReactElement;
  className?: string;
}): ReactElement {
  return (
    <Link href={href} className={cn(className)}>
      {children({
        className: cn(
          "h-4",
          "text-secondary",
          "hover:text-highlight-primary",
          "transition-colors",
          "duration-200"
        ),
      })}
    </Link>
  );
}
