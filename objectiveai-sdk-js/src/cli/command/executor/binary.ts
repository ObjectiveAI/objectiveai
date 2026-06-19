import type { CliCommandRequest } from "../request";

/** Construction options for {@link BinaryCommandExecutor}. */
export interface BinaryCommandExecutorOptions {
  /** Layout root; the binary is `<objectiveaiDir>/bin/objectiveai[.exe]`. */
  objectiveaiDir?: string;
  /** Extra environment variables layered onto the child's env. */
  extraEnv?: Record<string, string>;
  /** Kill the child when the stream is dropped (default false). */
  killOnDrop?: boolean;
  /** Detach the child so it outlives the parent (default false). */
  detach?: boolean;
}

// Binary executor: spawns the objectiveai CLI as a child process and streams
// its stdout (line-delimited JSON, yielded raw — the caller's `CliStream`
// discriminates errors vs responses). The request is serialized to JSON and
// passed via the cli's top-level `--request` flag; stdin is null, stdout is
// piped, stderr is inherited.
//
// Node modules are imported dynamically so this module stays importable in
// non-Node environments; the dynamic import only runs when `execute` is
// actually called.
export class BinaryCommandExecutor {
  private readonly options: BinaryCommandExecutorOptions;

  constructor(options: BinaryCommandExecutorOptions = {}) {
    this.options = options;
  }

  async *execute(request: CliCommandRequest): AsyncGenerator<unknown> {
    const argv = ["--request", JSON.stringify(request)];

    const { spawn } = await import("node:child_process");
    const os = await import("node:os");
    const path = await import("node:path");
    const readline = await import("node:readline");

    const binary = this.resolveBinary(os.homedir(), path.join);

    const child = spawn(binary, argv, {
      stdio: ["ignore", "pipe", "inherit"],
      env: { ...process.env, ...(this.options.extraEnv ?? {}) },
      detached: this.options.detach ?? false,
    });
    if (this.options.detach) child.unref();

    const stdout = child.stdout;
    if (stdout === null) {
      throw new Error("binary executor: child stdout was not piped");
    }

    const rl = readline.createInterface({ input: stdout, crlfDelay: Infinity });
    try {
      for await (const line of rl) {
        if (line.trim().length === 0) continue;
        yield JSON.parse(line);
      }
    } finally {
      rl.close();
      if (this.options.killOnDrop) child.kill();
    }
  }

  private resolveBinary(
    homedir: string,
    join: (...parts: string[]) => string,
  ): string {
    const exe = process.platform === "win32" ? "objectiveai.exe" : "objectiveai";
    const dir = this.options.objectiveaiDir ?? join(homedir, ".objectiveai");
    return join(dir, "bin", exe);
  }
}
