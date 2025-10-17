#!/usr/bin/env -S bun run
/**
 * Various commands for interacting with the remote server.
 */
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
const DB_PATH_REMOTE = "/home/cookie-odyssey/app/db.sqlite";
const DB_PATH_BACKUP_REMOTE = "/tmp/cookie-odyssey.sqlite.bak";

// eslint-disable-next-line unicorn/template-indent -- No.
const DB_BACKUP_SCRIPT = `
set -eux
sqlite3 "${DB_PATH_REMOTE}" ".backup ${DB_PATH_BACKUP_REMOTE}"
chmod a+r "${DB_PATH_BACKUP_REMOTE}"
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
  options?: SSHExecCommandOptions,
  displayScript?: string,
): Promise<SSHExecCommandResponse> {
  logger.debug(`Running command:\n${displayScript ?? command}\n`);
  try {
    var result = await ssh.execCommand(command, options);
  } catch (error) {
    throw new Error(`Failed to run command:\n${command}`, { cause: error });
  }
  if (result.code !== 0) {
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
    `tmpfile=$(mktemp) && cat > "$tmpfile" && echo "$tmpfile"`,
    { stdin: script },
  );

  const remoteScriptPath = result.stdout.trim();

  const command = options?.user
    ? `su - ${options.user} -c "bash '${remoteScriptPath}'"`
    : `bash '${remoteScriptPath}'`;
  return await sshExec(ssh, command, undefined, script);
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
      await ssh.putFile(DB_PATH_LOCAL, DB_PATH_BACKUP_REMOTE);
      logger.info(`Database pushed to ${DB_PATH_BACKUP_REMOTE}.`);
      // TODO: Copy it to the right place, and get rid of `wal`/`shm` files.
    });
  });

  await program.parseAsync();
}

await main();
