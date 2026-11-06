// phronia-shim: opencode@v2
// shim-version: 2611.5.1
// build-id: 841f4ad38eec
// content-hash: 3846b2d12c3b04e46b9bdd5b2389a5e3c38ec4dbd8a50f022056ad9daa468a7a
// DO NOT EDIT. Regenerate with: pnpm build:shims

import { createRequire as __phroniaCreateRequire } from "node:module"; const require = __phroniaCreateRequire(import.meta.url);

// packages/cli/src/templates/opencode/adapters/v2/plugin.js
import { createHash } from "node:crypto";

// packages/cli/src/templates/opencode/adapters/v2/lib/debug.js
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
  const msg = `[${timestamp}] [${prefix}] ${args.map((value) => typeof value === "object" ? JSON.stringify(value) : value).join(" ")}`;
  try {
    appendFileSync(DEBUG_LOG, `${msg}
`);
  } catch {
  }
}

// packages/cli/src/templates/opencode/adapters/v2/lib/phronia-client.js
import { spawn } from "node:child_process";
function resolveCli() {
  const explicit = process.env.PHRONIA_CLI_PATH;
  if (explicit) {
    return explicit.endsWith(".py") ? { command: process.env.PYTHON || "python3", args: [explicit] } : { command: process.execPath, args: [explicit] };
  }
  return { command: process.platform === "win32" ? "phronia.cmd" : "phronia", args: [] };
}
function contextArgs(cli, { event, projectRoot, adapter }) {
  return [
    ...cli.args,
    "context",
    "--event",
    event,
    "--platform",
    "opencode",
    ...adapter ? ["--adapter", adapter] : [],
    "--project-root",
    projectRoot,
    "--input",
    "-",
    "--json"
  ];
}
function spawnContext(cli, args, input, timeoutMs) {
  return new Promise((resolve, reject) => {
    const child = spawn(cli.command, args, {
      stdio: ["pipe", "pipe", "pipe"],
      env: process.env,
      windowsHide: true,
      // `.cmd` shims need cmd.exe on Windows; POSIX runs argv directly.
      shell: process.platform === "win32"
    });
    let stdout = "";
    let stderr = "";
    let settled = false;
    const finish = (error, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (error) reject(error);
      else resolve(value);
    };
    const timer = setTimeout(() => {
      child.kill();
      finish(new Error(`phronia context timed out after ${timeoutMs}ms`));
    }, timeoutMs);
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", (error) => finish(error));
    child.on("close", (code) => {
      if (code !== 0) {
        finish(new Error(stderr.trim() || `phronia context exited ${code}`));
        return;
      }
      try {
        finish(null, JSON.parse(stdout));
      } catch (error) {
        finish(new Error(`phronia context returned invalid JSON: ${error.message}`));
      }
    });
    child.stdin.on("error", () => {
    });
    child.stdin.end(JSON.stringify(input ?? {}));
  });
}
function adapterUnsupported(error) {
  return /unknown option.*--adapter|--adapter.*unknown|unknown argument.*--adapter/i.test(
    error?.message ?? ""
  );
}
async function runContext({
  event,
  projectRoot,
  input,
  adapter = "opencode@v2",
  timeoutMs = 8e3
}) {
  const cli = resolveCli();
  try {
    return await spawnContext(
      cli,
      contextArgs(cli, { event, projectRoot, adapter }),
      input,
      timeoutMs
    );
  } catch (error) {
    if (!adapter || !adapterUnsupported(error)) throw error;
    return spawnContext(
      cli,
      contextArgs(cli, { event, projectRoot, adapter: null }),
      input,
      timeoutMs
    );
  }
}
function blocksText(response) {
  if (!response || !Array.isArray(response.blocks)) return "";
  return response.blocks.map((block) => block.text).join("\n\n");
}

// packages/cli/src/templates/opencode/adapters/v2/plugin.js
var ADAPTER_ID = "opencode@v2";
var PHRONIA_SUBAGENT_RE = /^(?:phronia|trellis)-(?:implement|check|research)$/i;
var FIRST_REPLY_NOTICE_RE = /<first-reply-notice>[\s\S]*?<\/first-reply-notice>\s*/g;
var ACTIVE_TASK_HINT_RE = /^\s*Active task:\s*(\S+)\s*$/m;
var MAX_CACHE_ENTRIES = 32;
function disabled() {
  return process.env.PHRONIA_HOOKS === "0" || process.env.PHRONIA_DISABLE_HOOKS === "1" || process.env.OPENCODE_NON_INTERACTIVE === "1";
}
function createCache() {
  return {
    sessionStart: /* @__PURE__ */ new Map(),
    prompt: /* @__PURE__ */ new Map(),
    inflight: /* @__PURE__ */ new Map()
  };
}
function remember(map, key, value) {
  map.set(key, value);
  if (map.size > MAX_CACHE_ENTRIES) {
    map.delete(map.keys().next().value);
  }
}
function singleFlight(cache, key, factory) {
  const existing = cache.inflight.get(key);
  if (existing) return existing;
  const pending = Promise.resolve().then(factory).finally(() => cache.inflight.delete(key));
  cache.inflight.set(key, pending);
  return pending;
}
function isSubagent(agent) {
  return typeof agent === "string" && PHRONIA_SUBAGENT_RE.test(agent.trim());
}
function normalizeAgent(agent) {
  if (typeof agent !== "string") return null;
  const match = PHRONIA_SUBAGENT_RE.exec(agent.trim());
  return match ? match[0].replace(/^(?:phronia|trellis)-/i, "").toLowerCase() : null;
}
function denyError(mutation) {
  const next = Array.isArray(mutation.next) ? mutation.next.join("\n") : "";
  const error = new Error(`[${mutation.code}] ${mutation.reason}
${next}`);
  error.name = "PhroniaDispatchDenied";
  return error;
}
function messageText(message) {
  if (!message) return "";
  if (typeof message.content === "string") return message.content;
  if (!Array.isArray(message.content)) return "";
  return message.content.filter((part) => part?.type === "text" && typeof part.text === "string").map((part) => part.text).join("\n\n");
}
function lastUserMessage(messages) {
  if (!Array.isArray(messages)) return null;
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    if (messages[index]?.role === "user") return messages[index];
  }
  return null;
}
function lastUserText(messages) {
  return messageText(lastUserMessage(messages));
}
function hasAssistantMessage(messages) {
  return Array.isArray(messages) && messages.some((message) => message?.role === "assistant");
}
function compactionGeneration(messages) {
  if (!Array.isArray(messages)) return 0;
  let count = 0;
  for (const message of messages) {
    if (!Array.isArray(message?.content)) continue;
    for (const part of message.content) {
      if (part?.type === "compaction") count += 1;
    }
  }
  return count;
}
function hashText(text) {
  return createHash("sha256").update(String(text)).digest("hex").slice(0, 24);
}
function stripFirstReplyNotice(text) {
  return text.replace(FIRST_REPLY_NOTICE_RE, "");
}
function activeTaskHint(prompt) {
  if (typeof prompt !== "string" || !prompt) return null;
  const match = prompt.match(ACTIVE_TASK_HINT_RE);
  return match ? match[1].trim() : null;
}
async function onContext(ctx, event, cache) {
  if (disabled() || !event || isSubagent(event.agent)) return;
  const sessionID = event.sessionID;
  if (typeof sessionID !== "string" || !sessionID) return;
  const messages = Array.isArray(event.messages) ? event.messages : [];
  const prompt = lastUserText(messages);
  const generation = compactionGeneration(messages);
  const sessionKey = `${sessionID}#${generation}`;
  let sessionContext = cache.sessionStart.get(sessionKey);
  if (sessionContext === void 0) {
    try {
      const response = await singleFlight(
        cache,
        `s:${sessionKey}`,
        () => runContext({
          event: "session-start",
          projectRoot: ctx.location.directory,
          adapter: ADAPTER_ID,
          input: { session_id: sessionID, prompt }
        })
      );
      sessionContext = blocksText(response);
      if (hasAssistantMessage(messages)) {
        sessionContext = stripFirstReplyNotice(sessionContext);
      }
      remember(cache.sessionStart, sessionKey, sessionContext);
    } catch (error) {
      debugLog("v2-context", "session-start failed:", error.message);
    }
  }
  if (sessionContext && Array.isArray(event.system)) {
    event.system.push({ type: "text", text: sessionContext });
  }
  if (!prompt) return;
  const user = lastUserMessage(messages);
  const promptKey = `${sessionID}#${user?.id ?? hashText(prompt)}`;
  let breadcrumb = cache.prompt.get(promptKey);
  if (breadcrumb === void 0) {
    try {
      const response = await singleFlight(
        cache,
        `p:${promptKey}`,
        () => runContext({
          event: "user-prompt",
          projectRoot: ctx.location.directory,
          adapter: ADAPTER_ID,
          input: { session_id: sessionID, prompt }
        })
      );
      breadcrumb = blocksText(response);
      remember(cache.prompt, promptKey, breadcrumb);
    } catch (error) {
      debugLog("v2-context", "user-prompt failed:", error.message);
    }
  }
  if (breadcrumb && Array.isArray(event.messages)) {
    event.messages.push({
      role: "system",
      content: [{ type: "text", text: breadcrumb }]
    });
  }
}
async function onToolBefore(ctx, event) {
  if (disabled()) return;
  if (!event || event.tool !== "subagent") return;
  const input = event.input;
  if (!input || typeof input !== "object") return;
  const agentType = normalizeAgent(input.agent);
  if (!agentType) return;
  const originalPrompt = typeof input.prompt === "string" ? input.prompt : "";
  try {
    const response = await runContext({
      event: "subagent-spawn",
      projectRoot: ctx.location.directory,
      adapter: ADAPTER_ID,
      input: {
        session_id: event.sessionID,
        agent_type: agentType,
        prompt: originalPrompt,
        ...activeTaskHint(originalPrompt) ? { task_ref: activeTaskHint(originalPrompt) } : {}
      }
    });
    const mutation = Array.isArray(response?.mutations) ? response.mutations[0] : null;
    if (mutation?.op === "deny") throw denyError(mutation);
    if (mutation?.op === "set-prompt" && typeof mutation.value === "string" && mutation.value) {
      input.prompt = mutation.value;
    }
  } catch (error) {
    if (error?.name === "PhroniaDispatchDenied") throw error;
    debugLog("v2-tool", "subagent context failed:", error.message);
  }
}
var plugin_default = {
  id: "phronia.opencode.v2",
  async setup(ctx) {
    if (disabled()) return;
    const cache = createCache();
    const registrations = [];
    const register = async (label, factory) => {
      try {
        const registration = await factory();
        if (registration) registrations.push(registration);
      } catch (error) {
        debugLog("v2-setup", `${label} registration failed:`, error.message);
      }
    };
    if (ctx?.session?.hook) {
      await register(
        "session.context",
        () => ctx.session.hook("context", (event) => onContext(ctx, event, cache))
      );
    }
    if (ctx?.tool?.hook) {
      await register(
        "tool.execute.before",
        () => ctx.tool.hook("execute.before", (event) => onToolBefore(ctx, event))
      );
    }
    if (ctx?.shell?.hook) {
      await register(
        "shell.create.before",
        () => ctx.shell.hook("create.before", (event) => {
          event.env.PHRONIA_HOST = "opencode";
        })
      );
    }
    if (registrations.length === 0) {
      debugLog("v2-setup", "no compatible OpenCode v2 hooks found; plugin is inert");
    }
    return async () => {
      for (const registration of registrations) {
        try {
          await registration?.dispose?.();
        } catch {
        }
      }
    };
  }
};
export {
  plugin_default as default
};
