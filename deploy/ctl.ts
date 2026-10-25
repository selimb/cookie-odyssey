#!/usr/bin/env -S bun run
/**
 * Various commands for interacting with the remote server.
 */
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import * as commander from "@commander-js/extra-typings";
import { inspect } from "bun";
import {
  NodeSSH,
  type SSHExecCommandOptions,
  type SSHExecCommandResponse,
} from "node-ssh";
import SSHConfig from "ssh-config";
import { z } from "zod";

const REMOTE_HOST = "cookie-odyssey";

const DB_PATH_LOCAL = "data/prod.sqlite";
const DB_PATH_REMOTE = "/home/cookie-odyssey/data/db.sqlite";
const SERVICE = "cookie-odyssey-app.service";
const DB_PATH_BACKUP_REMOTE = "/tmp/cookie-odyssey.sqlite.bak";
// Separate from the backup, since that file is owned by the app user and so
// cannot be overwritten over SFTP.
const DB_PATH_PUSH_REMOTE = "/tmp/cookie-odyssey.sqlite.push";

// eslint-disable-next-line unicorn/template-indent -- No.
const DB_BACKUP_SCRIPT = `
set -eux
sqlite3 "${DB_PATH_REMOTE}" ".backup ${DB_PATH_BACKUP_REMOTE}"
chmod a+r "${DB_PATH_BACKUP_REMOTE}"
`.trim();

// Replaces the remote database with the one uploaded to DB_PATH_PUSH_REMOTE.
// Must run as the app user, with the service stopped.
// The `-wal` and `-shm` files belong to the old database, so they are removed.
// The file is staged next to the target so that the final mv is atomic.
// eslint-disable-next-line unicorn/template-indent -- No.
const DB_PUSH_SCRIPT = `
set -eux
DB="${DB_PATH_REMOTE}"
cp "${DB_PATH_PUSH_REMOTE}" "$DB.push"
rm -f "$DB-wal" "$DB-shm"
mv -f "$DB.push" "$DB"
`.trim();

const logger = console;

const zHostConfig = z.object({
  Hostname: z.string(),
  user: z.string(),
  IdentityAgent: z.string(),
});
type HostConfig = z.infer<typeof zHostConfig>;

async function getHostConfig(): Promise<HostConfig> {
  const sshConfig = SSHConfig.parse(
    await Bun.file(path.join(os.homedir(), ".ssh/config")).text(),
  );

  const hostConfigRaw = sshConfig.compute(REMOTE_HOST);
  try {
    return zHostConfig.parse(hostConfigRaw);
  } catch (error) {
    throw new Error(`Failed to parse SSH config:\n${inspect(hostConfigRaw)}`, {
      cause: error,
    });
  }
}

async function sshConnect<T>(fn: (ssh: NodeSSH) => Promise<T>): Promise<T> {
  const hostConfig = await getHostConfig();

  const ssh = new NodeSSH();
  const agent = hostConfig.IdentityAgent.replace("~", os.homedir());
  await ssh.connect({
    host: hostConfig.Hostname,
    username: hostConfig.user,
    agent,
  });
  try {
    return await fn(ssh);
  } finally {
    ssh.dispose();
  }
}

async function sshExec(
  ssh: NodeSSH,
  command: string,
  options?: SSHExecCommandOptions & { checkCode?: boolean },
  displayScript?: string,
): Promise<SSHExecCommandResponse> {
  logger.debug(`Running command:\n${displayScript ?? command}\n`);
  const { checkCode = true, ...sshOptions } = options ?? {};
  try {
    var result = await ssh.execCommand(command, sshOptions);
  } catch (error) {
    throw new Error(`Failed to run command:\n${command}`, { cause: error });
  }
  if (checkCode && result.code !== 0) {
    const lines = [
      `Command failed with code ${result.code}.`,
      "# command:",
      command,
      ...(displayScript ? ["# script:", displayScript] : []),
      "# stdout:",
      result.stdout,
      "# stderr:",
      result.stderr,
    ];
    throw new Error(lines.join("\n"));
  }
  return result;
}

async function sshExecScript(
  ssh: NodeSSH,
  script: string,
  options?: { user?: string },
): Promise<SSHExecCommandResponse> {
  // Write script to the remote.
  const result = await sshExec(
    ssh,
    `tmpfile=$(mktemp) && cat > "$tmpfile" && chmod 777 "$tmpfile" && echo "$tmpfile"`,
    { stdin: script },
  );

  const remoteScriptPath = result.stdout.trim();

  const command = options?.user
    ? `su - ${options.user} -c "bash '${remoteScriptPath}'"`
    : `bash '${remoteScriptPath}'`;
  return await sshExec(ssh, command, undefined, script);
}

/**
 * Writes a consistent snapshot of the local database to a new temporary file.
 * Unlike copying the file, `.backup` includes whatever is still in the `-wal`
 * file.
 */
async function snapshotLocalDb(): Promise<string> {
  if (!fs.existsSync(DB_PATH_LOCAL)) {
    throw new Error(`${DB_PATH_LOCAL} does not exist.`);
  }
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "cookie-odyssey-push-"));
  const snapshot = path.join(dir, "db.sqlite");
  await Bun.$`sqlite3 ${DB_PATH_LOCAL} ${`.backup '${snapshot}'`}`;
  const check = (
    await Bun.$`sqlite3 ${snapshot} "PRAGMA integrity_check"`.text()
  ).trim();
  if (check !== "ok") {
    throw new Error(`Integrity check failed for ${DB_PATH_LOCAL}:\n${check}`);
  }
  return snapshot;
}

async function main(): Promise<void> {
  const program = commander.program;
  program.command("db-pull").action(async () => {
    await sshConnect(async (ssh) => {
      await sshExecScript(ssh, DB_BACKUP_SCRIPT, { user: "cookie-odyssey" });
      await ssh.getFile(DB_PATH_LOCAL, DB_PATH_BACKUP_REMOTE);
      logger.info(`Database pulled to ${DB_PATH_LOCAL}.`);
    });
  });

  program.command("db-push").action(async () => {
    await sshConnect(async (ssh) => {
      const state = (
        await sshExec(ssh, `systemctl is-active "${SERVICE}"`, {
          // A non-zero exit code (3) is expected if the service is inactive or failed.
          checkCode: false,
        })
      ).stdout.trim();
      if (state !== "inactive" && state !== "failed") {
        throw new Error(
          `The service is ${state}. Stop it first, then run db-push again:\n  sudo systemctl stop ${SERVICE}`,
        );
      }

      const snapshot = await snapshotLocalDb();
      try {
        await ssh.putFile(snapshot, DB_PATH_PUSH_REMOTE);
      } finally {
        fs.rmSync(path.dirname(snapshot), { recursive: true, force: true });
      }
      // The app user needs to read it.
      await sshExec(ssh, `chmod a+r "${DB_PATH_PUSH_REMOTE}"`);
      await sshExecScript(ssh, DB_PUSH_SCRIPT, { user: "cookie-odyssey" });
      await sshExec(ssh, `rm -f "${DB_PATH_PUSH_REMOTE}"`);
      logger.info(
        `Database pushed. Start the service:\n  sudo systemctl start ${SERVICE}`,
      );
    });
  });

  await program.parseAsync();
}

await main();
