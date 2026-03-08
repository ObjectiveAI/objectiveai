import { defineConfig, Options } from "tsup";
import path from "path";

const baseConfig: Options = {
  entry: ["src/index.ts"],
  dts: true,
  outDir: "dist",
  treeshake: true,
  target: "es2020",
  external: ["react", "react-dom"],
  esbuildOptions(options) {
    options.alias = {
      src: path.resolve(__dirname, "src"),
    };
  },
};

export default defineConfig([
  {
    ...baseConfig,
    format: ["esm"],
    outExtension: () => ({ js: ".js" }),
    clean: true,
    // Copy CSS to dist after ESM build
    async onSuccess() {
      const { cpSync, existsSync } = await import("fs");
      const srcCss = path.resolve(__dirname, "src", "styles", "function-tree.css");
      const distCss = path.resolve(__dirname, "dist", "function-tree.css");
      if (existsSync(srcCss)) {
        cpSync(srcCss, distCss);
        console.log("Copied function-tree.css to dist/");
      }
    },
  },
  {
    ...baseConfig,
    format: ["cjs"],
    outExtension: () => ({ js: ".cjs" }),
    clean: false,
    dts: false,
  },
]);
