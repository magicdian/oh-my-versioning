/* global process */
// phronia-shim: zcode@v1
// shim-version: 0.6.17
// DO NOT EDIT. Regenerate with: phronia update --regenerate-shims
import { spawnSync } from "node:child_process"

// Global ZCode plugin fallback. No cwd search: ZCode must pass projectRoot or
// ZCODE_PROJECT_DIR, otherwise the bridge refuses instead of guessing.
const EVENTS = {
  SessionStart: "session-start",
  UserPromptSubmit: "user-prompt",
  PreToolUse: "pre-tool-use",
}

export default async function phroniaBridge({ event, payload, projectRoot } = {}) {
  const root = projectRoot || payload?.project_dir || process.env.ZCODE_PROJECT_DIR
  if (!root) {
    process.stderr.write("phronia-bridge: project root not provided; refusing to guess\n")
    return { error: "phronia-bridge:no-project-root" }
  }
  const cli = process.env.PHRONIA_CLI_PATH
  const args = cli ? [cli, "context"] : ["context"]
  args.push("--event", EVENTS[event] ?? event, "--platform", "zcode",
    "--project-root", root, "--input", "-", "--render")
  const result = spawnSync(cli ? process.execPath : "phronia", args, {
    input: JSON.stringify(payload ?? {}), encoding: "utf8", timeout: 15000,
  })
  if (result.status !== 0) {
    process.stderr.write(result.stderr || "phronia-bridge: context failed\n")
    return { error: "phronia-bridge:context-failed" }
  }
  try {
    return JSON.parse(result.stdout)
  } catch {
    return { additionalContext: result.stdout }
  }
}
