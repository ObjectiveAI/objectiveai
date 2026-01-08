import type { MetadataRoute } from "next";

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    item("docs"),
    item("docs/sdks"),
    item("docs/api/get/ensemble_llms"),
    item("docs/api/get/ensemble_llms/id"),
    item("docs/api/get/ensemble_llms/id/usage"),
    item("docs/api/get/ensembles"),
    item("docs/api/get/ensembles/id"),
    item("docs/api/get/ensembles/id/usage"),
    item("docs/api/get/functions"),
    item("docs/api/get/functions/fauthor/fid/version"),
    item("docs/api/get/functions/fauthor/fid/version/usage"),
    item("docs/api/post/functions"),
    item("docs/api/post/functions/fauthor/fid/version"),
    item("docs/api/post/functions/fauthor/fid/version/publish"),
    item("docs/api/get/functions/profiles"),
    item("docs/api/get/functions/fauthor/fid/profiles/pauthor/pid/version"),
    item(
      "docs/api/get/functions/fauthor/fid/profiles/pauthor/pid/version/usage"
    ),
    item("docs/api/post/functions/fauthor/fid/profiles/publish"),
    item("docs/api/post/functions/fauthor/fid/profiles/compute"),
    item("docs/api/post/vector/completions"),
    item("docs/api/post/chat/completions"),
    item("docs/api/get/auth/credits"),
    item("docs/api/get/auth/username"),
    item("docs/api/post/auth/username"),
    item("docs/api/get/auth/keys"),
    item("docs/api/post/auth/keys"),
    item("docs/api/delete/auth/keys"),
    item("docs/api/get/auth/keys/openrouter"),
    item("docs/api/post/auth/keys/openrouter"),
    item("docs/api/delete/auth/keys/openrouter"),
    item("terms"),
    item("privacy"),
  ];
}

function item(path?: string): MetadataRoute.Sitemap[number] {
  return {
    url: `${process.env.NEXT_PUBLIC_BASE_URL}${path ? `/${path}` : ""}`,
    lastModified: new Date(),
  };
}
