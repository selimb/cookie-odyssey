/** Runs all the damn build watcher processes. */
import concurrently from "concurrently";

concurrently(
  [
    { command: "just dev-server", name: "server" },
    { command: "just dev-js", name: "js" },
  ],
  {},
);
