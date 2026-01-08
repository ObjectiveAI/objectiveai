import Cookies from "js-cookie";
import { useCallback, useEffect, useState } from "react";

let themeGlobal: "dark" | "light" | undefined = undefined;

export function useTheme(): {
  theme: "dark" | "light" | undefined;
  setTheme: (theme: "dark" | "light") => void;
} {
  const [themeState, setThemeState] = useState<"dark" | "light" | undefined>(
    themeGlobal
  );

  const setTheme = useCallback(
    (theme: "dark" | "light" | undefined, readonly?: boolean): void => {
      if (theme === themeState) return;
      setThemeState(theme);
      themeGlobal = theme;

      // prevent transition when changing theme
      document.body.classList.add("notransition");
      let themeValid: boolean;
      if (theme === "dark") {
        document.body.classList.remove("light");
        document.body.classList.add("dark");
        themeValid = true;
      } else if (theme === "light") {
        document.body.classList.remove("dark");
        document.body.classList.add("light");
        themeValid = true;
      } else {
        document.body.classList.remove("dark");
        document.body.classList.remove("light");
        themeValid = false;
      }

      // allow transitions again
      setTimeout(() => {
        document.body.classList.remove("notransition");
      });

      // if not readonly, store theme in cookie and notify localStorage
      if (!readonly) {
        if (themeValid) {
          Cookies.set("theme", theme!, {
            sameSite: "lax",
            expires: 365,
            secure: true,
            path: "/",
          });
        } else {
          Cookies.remove("theme", { path: "/" });
        }
        localStorage.setItem("theme-changed", Date.now().toString());
        window.dispatchEvent(
          new StorageEvent("storage-local", {
            key: "theme-changed",
          })
        );
      }
    },
    [themeState]
  );

  useEffect(() => {
    const updateTheme = () => {
      const cookieTheme = Cookies.get("theme") as "dark" | "light" | undefined;
      if (cookieTheme === "dark" || cookieTheme === "light") {
        setTheme(cookieTheme, true);
      } else if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
        setTheme("dark", false);
      } else if (window.matchMedia("(prefers-color-scheme: light)").matches) {
        setTheme("light", false);
      } else {
        setTheme(undefined, true);
      }
    };
    updateTheme();

    const listener = (e: StorageEvent) => {
      if (e.key === "theme-changed") {
        updateTheme();
      }
    };
    window.addEventListener("storage", listener, { passive: true });
    window.addEventListener("storage-local" as "storage", listener, {
      passive: true,
    });
    return () => {
      window.removeEventListener("storage", listener);
      window.removeEventListener("storage-local" as "storage", listener);
    };
  }, [setTheme]);

  return { theme: themeState, setTheme };
}
