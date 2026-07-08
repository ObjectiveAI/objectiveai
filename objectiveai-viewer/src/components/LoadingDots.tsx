import cn from "classnames";

/** The staggered-pulse ellipsis every per-box loading state shows.
 * Text-sized dots at the inherited font size, so containers don't
 * resize when loading resolves into content. */
export function LoadingDots({ marker }: { marker: string }) {
  return (
    <span
      {...{ [marker]: true }}
      className={cn(
        "inline-flex",
        "items-center",
        "justify-center",
        "gap-0.5",
        "text-info-dim",
      )}
    >
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          className={cn(
            "animate-pulse",
            i === 1 && "[animation-delay:300ms]",
            i === 2 && "[animation-delay:600ms]",
          )}
        >
          .
        </span>
      ))}
    </span>
  );
}
