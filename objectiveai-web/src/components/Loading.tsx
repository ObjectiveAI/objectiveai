import cn from "classnames";

export function LoadingDots({
  prefix,
  className,
}: {
  prefix?: string;
  className?: string;
}) {
  return (
    <>
      <span className={cn("inline-flex", className)}>
        {prefix && (
          <span className={cn("animate-[pulse_2s_ease-in-out_infinite]")}>
            {prefix}
          </span>
        )}
        <span className={cn("animate-[dots_1.5s_ease-in-out_infinite]")}>
          .
        </span>
        <span className={cn("animate-[dots_1.5s_ease-in-out_infinite_0.5s]")}>
          .
        </span>
        <span className={cn("animate-[dots_1.5s_ease-in-out_infinite_1s]")}>
          .
        </span>
      </span>
      <style jsx>
        {`
          @keyframes dots {
            0%,
            20% {
              opacity: 0;
            }
            40% {
              opacity: 1;
            }
            100% {
              opacity: 0;
            }
          }
          @keyframes pulse {
            0%,
            100% {
              opacity: 0.5;
            }
            50% {
              opacity: 1;
            }
          }
        `}
      </style>
    </>
  );
}
