/**
 * To be run on the VPS, as root.
 * Uploaded to the VPS (along with the build) by deploy.sh.
 *
 * The layout under /home/cookie-odyssey is described at [release-dir].
 *
 * Installs ~/cookie-odyssey.tar.gz as a new release, and switches `current`
 * to it.
 *
 * If the new release has pending migrations, the service is stopped, the
 * database is backed up to ~/backups, and the migrations are applied.
 *
 * Anything that fails switches back to the previous release and, if
 * migrations were involved, restores the database backup.
 * The service is stopped for the whole migration, so no writes are lost
 * before the new release starts.
 * If the new release then fails its health check, writes made in the few
 * seconds since it started are lost by the restore.
 *
 * Only Node and Bun built-ins are used, since there is no node_modules on the
 * VPS.
 *
 * Run it from the directory that it and the tarball were uploaded to (the home
 * directory of the operator).
 * The backups are written there too.
 *
 * Usage:
 *   sudo "$(mise which bun)" deploy-remote.ts
 *   sudo "$(mise which bun)" deploy-remote.ts --rollback
 */
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import readline from "node:readline/promises";

import { inspect } from "bun";

const SERVICE_USER = "cookie-odyssey";
const SERVICE = "cookie-odyssey-app.service";

// See [release-dir]
const BASE = "/home/cookie-odyssey";
const RELEASES = path.join(BASE, "releases");
const CURRENT = path.join(BASE, "current");
const PREVIOUS = path.join(BASE, "previous");
const DATA = path.join(BASE, "data");
const DB = path.join(DATA, "db.sqlite");

const RELEASES_KEEP = 5;
const BACKUPS_KEEP = 10;
const HEALTH_URL = "http://127.0.0.1:4444/";
const HEALTH_ATTEMPTS = 15;

// Every line of output is prefixed with where it comes from, as "(tag) ".
// This script's own messages use this tag.
const SELF_TAG = "deploy";
// Output of the app, whether run directly or through the service.
const APP_TAG = "cookie-odyssey";

type RunOptions = {
  cwd?: string;
  /** Prefix for the output of the command. Defaults to its name. */
  tag?: string;
  /** Run as this user, instead of root. */
  user?: string;
  /** Return stdout, which is then never printed. */
  capture?: boolean;
  /**
   * Print the output even if the command succeeds.
   * Otherwise it is only printed if the command fails.
   */
  show?: boolean;
  /** Return the exit code, instead of throwing on a non-zero one. */
  allowFailure?: boolean;
};

type RunResult = { code: number; stdout: string };

/** What has been done so far, and so what recovery has to undo. */
type State = {
  /** The release that was current at the start, if any. */
  prevRelease: string | undefined;
  /** Set once the database has been backed up. */
  backup: string | undefined;
  /** Set once the service has been stopped or switched to the new release. */
  touched: boolean;
};

class Declined extends Error {
  constructor() {
    super("Declined");
  }
}

/**
 * A signal arrived.
 * Handlers only record it, and it is acted upon once the current step has
 * finished, so recovery never overlaps with a running migration.
 */
let interrupted: string | undefined;
/** Once recovery has started, signals are ignored. */
let recovering = false;

// Writing can fail if the terminal is gone, which must not stop recovery.
function writeLine(sink: NodeJS.WriteStream, tag: string, line: string): void {
  try {
    sink.write(`(${tag}) ${line}\n`);
  } catch {
    // Ignored.
  }
}

function writeLines(sink: NodeJS.WriteStream, tag: string, text: string): void {
  if (text.trim() === "") {
    return;
  }
  for (const line of text.trimEnd().split("\n")) {
    writeLine(sink, tag, line);
  }
}

function log(message: string): void {
  writeLines(process.stdout, SELF_TAG, message);
}

function logError(message: string): void {
  writeLines(process.stderr, SELF_TAG, message);
}

function formatErr(error: unknown): string {
  return error instanceof Error ? error.message : inspect(error);
}

function throwIfInterrupted(): void {
  if (interrupted !== undefined && !recovering) {
    throw new Error(`Interrupted by ${interrupted}`);
  }
}

type OutputLine = { isStderr: boolean; text: string };

/**
 * Reads `stream` to the end and returns all of it.
 * Each line is also added to `lines` as soon as it is complete, so that lines
 * from stdout and stderr end up in roughly the order they were written.
 */
async function collect(
  stream: ReadableStream<Uint8Array>,
  isStderr: boolean,
  lines: OutputLine[],
): Promise<string> {
  const decoder = new TextDecoder();
  let all = "";
  let pending = "";
  const push = (text: string): void => {
    all += text;
    pending += text;
    const parts = pending.split("\n");
    pending = parts.pop() ?? "";
    for (const part of parts) {
      lines.push({ isStderr, text: part });
    }
  };
  const reader = stream.getReader();
  for (;;) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    push(decoder.decode(value, { stream: true }));
  }
  push(decoder.decode());
  if (pending !== "") {
    lines.push({ isStderr, text: pending });
  }
  return all;
}

async function run(
  cmd: string[],
  options: RunOptions = {},
): Promise<RunResult> {
  const fullCmd = options.user
    ? ["runuser", "-u", options.user, "--", ...cmd]
    : cmd;
  const proc = Bun.spawn(fullCmd, {
    cwd: options.cwd,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  const tag = options.tag ?? path.basename(cmd[0] ?? "");
  const lines: OutputLine[] = [];
  const [stdout, , code] = await Promise.all([
    collect(proc.stdout, false, lines),
    collect(proc.stderr, true, lines),
    proc.exited,
  ]);
  if (code !== 0 || options.show) {
    for (const { isStderr, text } of lines) {
      if (isStderr || !options.capture) {
        writeLine(isStderr ? process.stderr : process.stdout, tag, text);
      }
    }
  }
  throwIfInterrupted();
  if (code !== 0 && !options.allowFailure) {
    throw new Error(`Command failed with code ${code}: ${fullCmd.join(" ")}`);
  }
  return { code, stdout };
}

function isSymlink(p: string): boolean {
  try {
    return fs.lstatSync(p).isSymbolicLink();
  } catch {
    return false;
  }
}

/** Atomically points the symlink `linkPath` at the directory `target`. */
function link(target: string, linkPath: string): void {
  const tmp = `${linkPath}.tmp`;
  fs.rmSync(tmp, { force: true });
  fs.symlinkSync(target, tmp);
  fs.renameSync(tmp, linkPath);
}

/** Reads the VERSION file (see [VERSION]) as a key-value map. */
function parseVersion(text: string): Partial<Record<string, string>> {
  const result: Partial<Record<string, string>> = {};
  for (const line of text.split("\n")) {
    const index = line.indexOf("=");
    if (index > 0) {
      result[line.slice(0, index)] = line.slice(index + 1);
    }
  }
  return result;
}

function readVersion(releaseDir: string): string {
  return fs.readFileSync(path.join(releaseDir, "VERSION"), "utf8");
}

async function systemctl(
  action: string,
  options?: { allowFailure?: boolean },
): Promise<RunResult> {
  log(`Running systemctl ${action} ${SERVICE}`);
  return await run(["systemctl", action, SERVICE], options);
}

async function waitHealthy(): Promise<boolean> {
  log(`Running health check against ${HEALTH_URL}`);
  for (let attempt = 1; attempt <= HEALTH_ATTEMPTS; attempt++) {
    const prefix = `Health check ${attempt}/${HEALTH_ATTEMPTS}`;
    const active = await run(["systemctl", "is-active", "--quiet", SERVICE], {
      allowFailure: true,
    });
    if (active.code === 0) {
      try {
        // Redirects are followed, so this reaches the login page.
        const response = await fetch(HEALTH_URL, {
          signal: AbortSignal.timeout(2000),
        });
        if (response.ok) {
          log(`${prefix}: ok (HTTP ${response.status}).`);
          log("Health check passed.");
          return true;
        }
        log(`${prefix}: HTTP ${response.status}, retrying.`);
      } catch (error) {
        log(`${prefix}: ${formatErr(error)}, retrying.`);
      }
    } else {
      log(`${prefix}: service is not active, retrying.`);
    }
    await Bun.sleep(1000);
  }
  logError("Health check failed.");
  return false;
}

/**
 * Runs `cookie-odyssey args` the way the service would: as the service user,
 * from the data directory.
 */
async function runApp(
  bin: string,
  args: string[],
  options?: { capture?: boolean; show?: boolean },
): Promise<RunResult> {
  log(`Running ${path.basename(bin)} ${args.join(" ")}`);
  return await run([bin, ...args], {
    ...options,
    tag: APP_TAG,
    user: SERVICE_USER,
    cwd: DATA,
  });
}

/**
 * Backs up the database to `backups` in `dir`, owned like `dir`, and keeps the
 * newest.
 */
async function backupDb(dir: string): Promise<string> {
  let oldSha = "unknown";
  try {
    const sha = parseVersion(readVersion(CURRENT)).sha;
    if (sha !== undefined) {
      oldSha = sha.slice(0, 8);
    }
  } catch {
    // There is no current release, as on the first deploy.
  }

  const backups = path.join(dir, "backups");
  fs.mkdirSync(backups, { recursive: true });
  const timestamp = new Date()
    .toISOString()
    .replaceAll(/[-:]/g, "")
    .replace(/\.\d+Z$/, "Z");
  const file = path.join(backups, `db-${timestamp}-${oldSha}.sqlite`);
  log("Backing up the database.");
  await run(["sqlite3", DB, `.backup '${file}'`]);
  const owner = fs.statSync(dir);
  fs.chownSync(file, owner.uid, owner.gid);
  fs.chmodSync(file, 0o600);
  log(`Database backed up to ${file}`);

  const existing = fs
    .readdirSync(backups)
    .filter((name) => /^db-.*\.sqlite$/.test(name))
    .toSorted()
    .toReversed();
  for (const name of existing.slice(BACKUPS_KEEP)) {
    fs.rmSync(path.join(backups, name), { force: true });
  }
  return file;
}

/** Restores the database from `backup`. The service must be stopped. */
function restoreDb(backup: string): void {
  fs.rmSync(`${DB}-wal`, { force: true });
  fs.rmSync(`${DB}-shm`, { force: true });
  fs.copyFileSync(backup, DB);
  // The data directory is owned by the service user.
  const owner = fs.statSync(DATA);
  fs.chownSync(DB, owner.uid, owner.gid);
  log(`Database restored from ${backup}`);
}

/** Deletes all but the newest few releases, never the current or previous. */
function pruneReleases(): void {
  const keep = new Set<string>();
  for (const symlink of [CURRENT, PREVIOUS]) {
    if (isSymlink(symlink)) {
      keep.add(fs.realpathSync(symlink));
    }
  }
  const newestFirst = fs
    .readdirSync(RELEASES)
    .map((name) => ({
      dir: path.join(RELEASES, name),
      mtime: fs.statSync(path.join(RELEASES, name)).mtimeMs,
    }))
    .toSorted((a, b) => b.mtime - a.mtime);
  for (const { dir } of newestFirst.slice(RELEASES_KEEP)) {
    if (!keep.has(dir)) {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  }
}

/**
 * Switches back to the previous release, and the database if it was backed
 * up.
 * Leaves the service running, unless there is no previous release (the first
 * deploy), in which case there is nothing to run and the service is stopped.
 * Every step is attempted, even if an earlier one failed.
 */
async function recover(state: State, reason: string): Promise<void> {
  // Recovery must run to the end, even if the user interrupts it again.
  recovering = true;
  logError(`${reason}, recovering.`);

  const attempt = async (what: string, fn: () => unknown): Promise<void> => {
    try {
      await fn();
    } catch (error) {
      logError(`Recovery step failed (${what}): ${formatErr(error)}`);
    }
  };

  if (state.prevRelease === undefined) {
    await attempt("remove current", () => {
      fs.rmSync(CURRENT, { force: true });
    });
  } else {
    const prev = state.prevRelease;
    await attempt("switch to previous", () => {
      link(prev, CURRENT);
    });
  }
  if (state.backup !== undefined) {
    const { backup } = state;
    await attempt("stop service", async () => {
      await systemctl("stop");
    });
    await attempt("restore database", () => {
      restoreDb(backup);
    });
  }
  if (state.prevRelease === undefined) {
    logError("There is no previous release, leaving the service stopped.");
    await attempt("stop service", async () => {
      await systemctl("stop");
    });
  } else if (state.touched) {
    await attempt("restart service", async () => {
      await systemctl("restart");
    });
    if (!(await waitHealthy())) {
      logError("Previous release is not healthy either!");
    }
  }
}

async function confirm(question: string): Promise<boolean> {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  const prompt = `(${SELF_TAG}) ${question}`;
  // Without this, Ctrl-C at the prompt would leave readline waiting forever.
  const closed = new Promise<string>((resolve) => {
    rl.once("close", () => {
      resolve("");
    });
  });
  rl.on("SIGINT", () => {
    interrupted ??= "SIGINT";
    rl.close();
  });
  try {
    const answer = await Promise.race([rl.question(prompt), closed]);
    throwIfInterrupted();
    return answer === "y";
  } finally {
    rl.close();
  }
}

async function deploy(): Promise<void> {
  // Uploaded to the home directory of the operator, which is where this is
  // run from.
  const tarball = path.resolve("cookie-odyssey.tar.gz");
  if (!fs.existsSync(tarball)) {
    throw new Error(`${tarball} not found. Run deploy.sh first.`);
  }

  const staging = fs.mkdtempSync(path.join(os.tmpdir(), "cookie-odyssey-"));
  try {
    log(`Extracting ${tarball}`);
    await run(["tar", "-xzf", tarball, "-C", staging, "--no-same-owner"]);

    const state: State = {
      prevRelease: isSymlink(CURRENT) ? fs.realpathSync(CURRENT) : undefined,
      backup: undefined,
      touched: false,
    };

    // From here on, a failure or a signal (for example an SSH disconnect)
    // must not leave the service stopped or on a bad release.
    try {
      await deployRelease(tarball, staging, state);
    } catch (error) {
      await recover(state, formatErr(error));
      process.exitCode = 1;
    }
  } finally {
    fs.rmSync(staging, { recursive: true, force: true });
  }
}

async function deployRelease(
  tarball: string,
  staging: string,
  state: State,
): Promise<void> {
  if (state.prevRelease !== undefined) {
    log("Currently deployed:");
    log(readVersion(state.prevRelease));
  }
  const newVersion = readVersion(staging);
  log("Deploying:");
  log(newVersion);

  const version = parseVersion(newVersion);
  const runNumber = version.run_number;
  const sha = version.sha;
  if (runNumber === undefined || sha === undefined) {
    throw new Error("VERSION has no run_number or sha.");
  }
  const release = path.join(RELEASES, `${runNumber}-${sha.slice(0, 8)}`);
  if (release === state.prevRelease) {
    throw new Error(`${release} is already deployed.`);
  }

  // Install the release.
  // The service only needs to read it, so it is owned by root.
  log(`Installing release to ${release}`);
  fs.rmSync(release, { recursive: true, force: true });
  fs.mkdirSync(release, { recursive: true });
  await run(["cp", "-a", `${staging}/.`, `${release}/`]);
  await run(["chown", "-R", "root:root", release]);
  await run(["chmod", "-R", "u+rwX,go+rX,go-w", release]);
  const bin = path.join(release, "cookie-odyssey");

  // The new binary is the one that knows about the new migrations.
  // A failure here is not swallowed: run has already printed the error.
  log("Checking for pending migrations.");
  const status = await runApp(bin, ["migrate", "status"], { capture: true });
  const pending = status.stdout
    .split("\n")
    .filter((line) => line.startsWith("pending "));
  if (pending.length === 0) {
    log("No pending migrations.");
  } else {
    log("Pending migrations:");
    log(pending.join("\n"));
    if (!(await confirm("Stop the service and apply them? [y/N] "))) {
      throw new Declined();
    }

    state.touched = true;
    await systemctl("stop");
    state.backup = await backupDb(path.dirname(tarball));
    await runApp(bin, ["migrate", "up"], { show: true });
    log("Migrations applied.");
  }

  // Run the app's own checks (assets, templates, migrations, database,
  // storage) before switching, so that a bad release is rejected early.
  await runApp(bin, ["check"]);
  log("Release checks passed.");

  state.touched = true;
  log(`Switching current to ${release}`);
  link(release, CURRENT);
  await systemctl("restart");
  if (!(await waitHealthy())) {
    await run(["journalctl", "-u", SERVICE, "--no-pager", "-n", "30"], {
      allowFailure: true,
      show: true,
    });
    throw new Error("New release is not healthy");
  }

  // The deploy is good, so bookkeeping failures must not roll it back.
  recovering = true;
  log("Cleaning up.");
  try {
    if (state.prevRelease !== undefined) {
      link(state.prevRelease, PREVIOUS);
    }
    fs.rmSync(tarball, { force: true });
    pruneReleases();
  } catch (error) {
    logError(`Deployed, but cleanup failed: ${formatErr(error)}`);
  }
  log("Deployed.");
}

async function rollback(): Promise<void> {
  if (!isSymlink(CURRENT) || !isSymlink(PREVIOUS)) {
    throw new Error("No previous release.");
  }
  const from = fs.realpathSync(CURRENT);
  const to = fs.realpathSync(PREVIOUS);
  log("Rolling back to:");
  log(readVersion(to));

  try {
    await runApp(path.join(to, "cookie-odyssey"), ["check"]);
  } catch (error) {
    logError(formatErr(error));
    throw new Error(
      "Check failed. If the database has newer migrations than this " +
        "release, restore a database backup first.",
      { cause: error },
    );
  }

  const state: State = { prevRelease: from, backup: undefined, touched: true };
  try {
    log(`Switching current to ${to}`);
    link(to, CURRENT);
    await systemctl("restart");
    if (!(await waitHealthy())) {
      throw new Error("Rollback is not healthy");
    }
  } catch (error) {
    await recover(state, formatErr(error));
    process.exitCode = 1;
    return;
  }
  link(from, PREVIOUS);
  log("Rolled back.");
}

async function main(): Promise<void> {
  if (process.getuid?.() !== 0) {
    throw new Error("This script must be run as root. Use sudo.");
  }
  const args = process.argv.slice(2);
  const isRollback = args[0] === "--rollback";
  if (args.length > 1 || (args.length === 1 && !isRollback)) {
    throw new Error("Usage: deploy-remote.ts [--rollback]");
  }
  if (!fs.existsSync(DATA)) {
    throw new Error(`${DATA} does not exist. See Provisioning in README.md.`);
  }

  for (const signal of ["SIGHUP", "SIGINT", "SIGTERM"] as const) {
    process.on(signal, () => {
      interrupted ??= signal;
    });
  }

  await (isRollback ? rollback() : deploy());
}

try {
  await main();
} catch (error) {
  logError(formatErr(error));
  process.exitCode = 1;
}
