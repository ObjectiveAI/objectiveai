import type { MetadataRoute } from "next";

export default function sitemap(): MetadataRoute.Sitemap {
  const baseUrl = "https://objective-ai.io";

  const staticRoutes = [
    "",
    "/functions",
    "/people",
    "/docs",
    "/legal",
    "/account/keys",
    "/account/credits",
    "/legal/terms",
    "/legal/privacy",
    // API documentation pages
    "/docs/api/get/ensemble_llms",
    "/docs/api/get/ensemble_llms/id",
    "/docs/api/get/ensemble_llms/id/usage",
    "/docs/api/get/ensembles",
    "/docs/api/get/ensembles/id",
    "/docs/api/get/ensembles/id/usage",
    "/docs/api/get/functions",
    "/docs/api/get/functions/fowner/frepository/fcommit",
    "/docs/api/get/functions/fowner/frepository/fcommit/usage",
    "/docs/api/get/functions/fowner/frepository/fcommit/profiles/powner/prepository/pcommit",
    "/docs/api/get/functions/fowner/frepository/fcommit/profiles/powner/prepository/pcommit/usage",
    "/docs/api/get/functions/profiles",
    "/docs/api/get/functions/profiles/pairs",
    "/docs/api/get/functions/profiles/powner/prepository/pcommit",
    "/docs/api/get/functions/profiles/powner/prepository/pcommit/usage",
    "/docs/api/get/auth/credits",
    "/docs/api/get/auth/keys",
    "/docs/api/get/auth/keys/openrouter",
    "/docs/api/get/vector/completions/id",
    "/docs/api/get/vector/completions/cache",
    "/docs/api/post/functions",
    "/docs/api/post/functions/fowner/frepository/fcommit",
    "/docs/api/post/functions/fowner/frepository/fcommit/profiles/powner/prepository/pcommit",
    "/docs/api/post/functions/fowner/frepository/fcommit/profiles/compute",
    "/docs/api/post/functions/profiles/powner/prepository/pcommit",
    "/docs/api/post/functions/profiles/compute",
    "/docs/api/post/vector/completions",
    "/docs/api/post/chat/completions",
    "/docs/api/post/auth/keys",
    "/docs/api/post/auth/keys/openrouter",
    "/docs/api/delete/auth/keys",
    "/docs/api/delete/auth/keys/openrouter",
  ];

  const getPriority = (route: string): number => {
    if (route === "") return 1;
    if (route.startsWith("/docs/api/")) return 0.3;
    if (route.startsWith("/account/") || route.startsWith("/legal/")) return 0.3;
    if (["/legal", "/people", "/docs"].includes(route)) return 0.5;
    return 0.8;
  };

  return staticRoutes.map((route) => ({
    url: `${baseUrl}${route}`,
    lastModified: new Date(),
    changeFrequency: route === "" ? "weekly" : "monthly",
    priority: getPriority(route),
  }));
}
