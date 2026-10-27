/* global process */
// phronia-shim: opencode@v1
// shim-version: 0.6.17
// DO NOT EDIT. Regenerate with: phronia update --regenerate-shims
/**
 * Bridge to the Phronia core CLI.
 *
 * OpenCode plugins cannot import the CLI package directly, so they spawn the
 * same `phronia context` entrypoint every other harness uses. B6 replaces this
 * with an in-process bundle; the contract is unchanged.
 */

import { spawnSync } from "node:child_process"

export function resolveCli() {
  const explicit = process.env.PHRONIA_CLI_PATH
  if (explicit) {
    return explicit.endsWith(".py")
      ? { command: process.env.PYTHON || "python3", args: [explicit] }
      : { command: process.execPath, args: [explicit] }
  }
  return { command: process.platform === "win32" ? "phronia.cmd" : "phronia", args: [] }
}

export function runContext({ event, platform, projectRoot, input, render = false }) {
  const cli = resolveCli()
  const args = [
    ...cli.args,
    "context",
    "--event",
    event,
    "--platform",
    platform,
    "--project-root",
    projectRoot,
    "--input",
    "-",
  ]
  if (render) {
    args.push("--render")
  } else {
    args.push("--json")
  }
  const result = spawnSync(cli.command, args, {
    input: JSON.stringify(input ?? {}),
    encoding: "utf8",
    timeout: 15000,
    env: process.env,
  })
  if (result.error) {
    throw result.error
  }
  if (result.status !== 0) {
    throw new Error(result.stderr || `phronia context exited ${result.status}`)
  }
  return render ? result.stdout : JSON.parse(result.stdout)
}

export function blocksText(response) {
  if (!response || !Array.isArray(response.blocks)) {
    return ""
  }
  return response.blocks.map(block => block.text).join("\n\n")
}
