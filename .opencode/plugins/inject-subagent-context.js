// phronia-shim: opencode@v1
// shim-version: 2610.27.8
// build-id: ec26ef124d33
// content-hash: 8cb176cb8837a9d1bda4adbe7089ce918ca50e363110adc91d82db31840523e7
// DO NOT EDIT. Regenerate with: pnpm build:shims

import { createRequire as __phroniaCreateRequire } from "node:module"; const require = __phroniaCreateRequire(import.meta.url);

// packages/cli/src/templates/opencode/plugins/inject-subagent-context.js
import { createHash } from "node:crypto";

// packages/cli/src/templates/opencode/lib/debug.js
import { appendFileSync } from "node:fs";
import { platform } from "node:os";
import { join } from "node:path";
var DEBUG_LOG = join(
  process.env.TMPDIR || (platform() === "win32" ? "." : "/tmp"),
  "phronia-plugin-debug.log"
);
function debugLog(prefix, ...args) {
  if (!process.env.PHRONIA_DEBUG && !process.env.DEBUG) return;
  const timestamp = (/* @__PURE__ */ new Date()).toISOString();
  const msg = `[${timestamp}] [${prefix}] ${args.map((value) => typeof value === "object" ? JSON.stringify(value) : value).join(" ")}
`;
  try {
    appendFileSync(DEBUG_LOG, msg);
  } catch {
  }
}

// packages/cli/src/templates/opencode/lib/phronia-client.js
import { spawnSync } from "node:child_process";
function resolveCli() {
  const explicit = process.env.PHRONIA_CLI_PATH;
  if (explicit) {
    return explicit.endsWith(".py") ? { command: process.env.PYTHON || "python3", args: [explicit] } : { command: process.execPath, args: [explicit] };
  }
  return { command: process.platform === "win32" ? "phronia.cmd" : "phronia", args: [] };
}
function runContext({ event, platform: platform2, projectRoot, input, render = false }) {
  const cli = resolveCli();
  const args = [
    ...cli.args,
    "context",
    "--event",
    event,
    "--platform",
    platform2,
    "--project-root",
    projectRoot,
    "--input",
    "-"
  ];
  if (render) {
    args.push("--render");
  } else {
    args.push("--json");
  }
  const result = spawnSync(cli.command, args, {
    input: JSON.stringify(input ?? {}),
    encoding: "utf8",
    timeout: 15e3,
    env: process.env
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(result.stderr || `phronia context exited ${result.status}`);
  }
  return render ? result.stdout : JSON.parse(result.stdout);
}

// packages/cli/src/templates/opencode/plugins/inject-subagent-context.js
var AGENTS_ALL = ["implement", "check", "research"];
var ACTIVE_TASK_HINT_RE = /^\s*Active task:\s*(\S+)\s*$/m;
function sanitizeKey(raw) {
  const safe = String(raw ?? "").trim().replace(/[^A-Za-z0-9._-]+/g, "_").replace(/^[._-]+|[._-]+$/g, "");
  return safe ? safe.slice(0, 160) : "";
}
function hashValue(raw) {
  return createHash("sha256").update(String(raw)).digest("hex").slice(0, 24);
}
function stringValue(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}
function lookupString(input, keys) {
  if (!input || typeof input !== "object") return null;
  for (const key of keys) {
    const value = stringValue(input[key]);
    if (value) return value;
  }
  return null;
}
function contextKey(input) {
  const override = stringValue(process.env.PHRONIA_CONTEXT_ID);
  if (override) return sanitizeKey(override) || hashValue(override);
  const sessionId = lookupString(input, ["session_id", "sessionId", "sessionID"]);
  if (sessionId) {
    const safe = sanitizeKey(sessionId);
    return safe ? `opencode_${safe}` : `opencode_${hashValue(sessionId)}`;
  }
  const transcript = lookupString(input, ["transcript_path", "transcriptPath", "transcript"]);
  return transcript ? `opencode_transcript_${hashValue(transcript)}` : null;
}
function activeTaskHint(prompt) {
  if (typeof prompt !== "string" || !prompt) return null;
  const match = prompt.match(ACTIVE_TASK_HINT_RE);
  return match ? match[1].trim() : null;
}
function envValue(env, key) {
  const value = env?.[key];
  return typeof value === "string" && value ? value : null;
}
function shellBasename(value) {
  return value.replace(/\\/g, "/").split("/").pop()?.toLowerCase() || "";
}
function isWindowsPosixShell(env = process.env) {
  if (envValue(env, "MSYSTEM")) return true;
  if (envValue(env, "MINGW_PREFIX")) return true;
  if (envValue(env, "OPENCODE_GIT_BASH_PATH")) return true;
  const ostype = envValue(env, "OSTYPE")?.toLowerCase() || "";
  if (/(msys|mingw|cygwin)/.test(ostype)) return true;
  return /^(bash|sh|zsh)(\.exe)?$/.test(shellBasename(envValue(env, "SHELL") || ""));
}
function powershellQuote(value) {
  return `'${String(value).replace(/'/g, "''")}'`;
}
function shellQuote(value) {
  return `'${String(value).replace(/'/g, `'\\''`)}'`;
}
function contextPrefix(key, hostPlatform = process.platform, env = process.env) {
  if (hostPlatform === "win32" && !isWindowsPosixShell(env)) {
    return `$env:PHRONIA_CONTEXT_ID = ${powershellQuote(key)}; `;
  }
  return `export PHRONIA_CONTEXT_ID=${shellQuote(key)}; `;
}
function commandKey(args) {
  if (!args || typeof args !== "object") return null;
  if (typeof args.command === "string") return "command";
  if (typeof args.cmd === "string") return "cmd";
  return null;
}
function commandHasContext(command) {
  const first = command.trimStart().split(/[;&|]/, 1)[0].trimStart();
  return /^PHRONIA_CONTEXT_ID\s*=/.test(first) || /^export\s+PHRONIA_CONTEXT_ID\s*=/.test(first) || /^env\s+(?:(?:-\S+|[A-Za-z_][A-Za-z0-9_]*=\S*)\s+)*PHRONIA_CONTEXT_ID\s*=/.test(first) || /^\$env:PHRONIA_CONTEXT_ID\s*=/i.test(first);
}
function injectBashContext(input, output, hostPlatform, env) {
  const args = output?.args;
  const key = commandKey(args);
  if (!key) return false;
  const command = args[key];
  if (!command?.trim() || commandHasContext(command)) return false;
  const sessionKey = contextKey(input);
  if (!sessionKey) return false;
  args[key] = `${contextPrefix(sessionKey, hostPlatform, env)}${command}`;
  return true;
}
function normalizeAgentType(value) {
  const name = String(value || "").replace(/^phronia-/, "").toLowerCase();
  return AGENTS_ALL.includes(name) ? name : null;
}
var inject_subagent_context_default = async ({ directory, platform: hostPlatform = process.platform, env = process.env }) => {
  debugLog("inject", "Plugin loaded, directory:", directory);
  const hook = async (input, output) => {
    try {
      if (process.env.PHRONIA_HOOKS === "0" || process.env.PHRONIA_DISABLE_HOOKS === "1") {
        return;
      }
      const toolName = input?.tool?.toLowerCase();
      if (toolName === "bash") {
        if (injectBashContext(input, output, hostPlatform, env)) {
          debugLog("inject", "Injected PHRONIA_CONTEXT_ID into Bash command");
        }
        return;
      }
      if (toolName !== "task") return;
      const args = output?.args;
      if (!args) return;
      const agentType = normalizeAgentType(args.subagent_type);
      if (!agentType) {
        debugLog("inject", "Skipping - unsupported subagent_type");
        return;
      }
      const originalPrompt = typeof args.prompt === "string" ? args.prompt : "";
      const sessionId = lookupString(input, ["session_id", "sessionId", "sessionID"]);
      const hint = activeTaskHint(originalPrompt);
      const response = runContext({
        event: "subagent-spawn",
        platform: "opencode",
        projectRoot: directory,
        input: {
          ...sessionId ? { session_id: sessionId } : {},
          ...hint ? { task_ref: hint } : {},
          subagent_type: agentType,
          prompt: originalPrompt
        }
      });
      const mutation = Array.isArray(response?.mutations) ? response.mutations[0] : null;
      if (!mutation || typeof mutation.value !== "string" || !mutation.value) {
        debugLog("inject", "No context to inject");
        return;
      }
      args.prompt = mutation.value;
      debugLog("inject", "Injected context for", agentType, "length:", mutation.value.length);
    } catch (error) {
      debugLog("inject", "Error in tool.execute.before:", error.message, error.stack);
    }
  };
  return { "tool.execute.before": hook };
};
export {
  inject_subagent_context_default as default
};
