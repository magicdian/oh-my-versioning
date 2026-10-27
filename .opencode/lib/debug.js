/* global process */
// phronia-shim: opencode@v1
// shim-version: 0.6.17
// DO NOT EDIT. Regenerate with: phronia update --regenerate-shims
/**
 * Optional debug log shared by the OpenCode plugins. Writes nothing unless
 * PHRONIA_DEBUG is set, and never throws.
 */

import { appendFileSync } from "node:fs"
import { platform } from "node:os"
import { join } from "node:path"

export const DEBUG_LOG = join(
  process.env.TMPDIR || (platform() === "win32" ? "." : "/tmp"),
  "phronia-plugin-debug.log",
)

export function debugLog(prefix, ...args) {
  if (!process.env.PHRONIA_DEBUG && !process.env.DEBUG) return
  const timestamp = new Date().toISOString()
  const msg = `[${timestamp}] [${prefix}] ${args
    .map(value => (typeof value === "object" ? JSON.stringify(value) : value))
    .join(" ")}\n`
  try {
    appendFileSync(DEBUG_LOG, msg)
  } catch {
    // debug logging must never affect the host
  }
}
