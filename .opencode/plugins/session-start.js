// phronia-shim: opencode@v1
// shim-version: 2610.28.3
// build-id: ae29014ff865
// content-hash: 11b8405666fe8d870aa6a9d18bf3d49f225d38f5aef05ca1c6f7a360b5722cb7
// DO NOT EDIT. Regenerate with: pnpm build:shims

import { createRequire as __phroniaCreateRequire } from "node:module"; const require = __phroniaCreateRequire(import.meta.url);

// packages/cli/src/templates/opencode/lib/phronia-client.js
import { spawnSync } from "node:child_process";
function resolveCli() {
  const explicit = process.env.PHRONIA_CLI_PATH;
  if (explicit) {
    return explicit.endsWith(".py") ? { command: process.env.PYTHON || "python3", args: [explicit] } : { command: process.execPath, args: [explicit] };
  }
  return { command: process.platform === "win32" ? "phronia.cmd" : "phronia", args: [] };
}
function runContext({ event, platform, projectRoot, input, render = false }) {
  const cli = resolveCli();
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
function blocksText(response) {
  if (!response || !Array.isArray(response.blocks)) {
    return "";
  }
  return response.blocks.map((block) => block.text).join("\n\n");
}

// packages/cli/src/templates/opencode/lib/context-visibility.js
var CONTEXT_PART_KINDS = {
  sessionStart: { offset: 2n, slot: "0" },
  workflowState: { offset: 1n, slot: "1" }
};
var MAX_CONTEXT_PART_OFFSET = Object.values(CONTEXT_PART_KINDS).reduce(
  (maximum, definition) => definition.offset > maximum ? definition.offset : maximum,
  0n
);
var MESSAGES_TRANSFORM_HOOK = "experimental.chat.messages.transform";
var PHRONIA_SUBAGENT_RE = /^(?:phronia|trellis)-/i;
function isPhroniaSubagent(input) {
  if (!input || typeof input !== "object") return false;
  const agent = typeof input.agent === "string" ? input.agent.trim() : "";
  return PHRONIA_SUBAGENT_RE.test(agent);
}
function findLatestUserMessageIndex(messages) {
  if (!Array.isArray(messages)) return -1;
  for (let i = messages.length - 1; i >= 0; i--) {
    if (messages[i]?.info?.role === "user") return i;
  }
  return -1;
}
function platformInputFromMessages(messages) {
  const index = findLatestUserMessageIndex(messages);
  if (index < 0) return null;
  const info = messages[index].info;
  if (!info || typeof info !== "object") return null;
  return {
    sessionID: info.sessionID,
    agent: info.agent
  };
}
function transcriptHasAssistantMessage(messages) {
  if (!Array.isArray(messages)) return false;
  return messages.some((message) => message?.info?.role === "assistant");
}
function prependEphemeralText(messages, text) {
  if (!Array.isArray(messages)) return false;
  if (typeof text !== "string") return false;
  const index = findLatestUserMessageIndex(messages);
  if (index < 0) return false;
  const original = messages[index];
  const parts = Array.isArray(original.parts) ? original.parts.slice() : [];
  parts.unshift({
    type: "text",
    text,
    synthetic: true
  });
  messages[index] = {
    ...original,
    parts
  };
  return true;
}

// packages/cli/src/templates/opencode/plugins/session-start.js
var FIRST_REPLY_NOTICE_RE = /<first-reply-notice>[\s\S]*?<\/first-reply-notice>\s*/g;
function stripFirstReplyNotice(context) {
  return context.replace(FIRST_REPLY_NOTICE_RE, "");
}
var session_start_default = async ({ directory }) => {
  return {
    [MESSAGES_TRANSFORM_HOOK]: async (_input, output) => {
      const messages = output?.messages;
      const platformInput = platformInputFromMessages(messages);
      if (!platformInput || isPhroniaSubagent(platformInput)) {
        return;
      }
      if (process.env.PHRONIA_HOOKS === "0" || process.env.PHRONIA_DISABLE_HOOKS === "1") {
        return;
      }
      if (process.env.OPENCODE_NON_INTERACTIVE === "1") {
        return;
      }
      const response = runContext({
        event: "session-start",
        platform: "opencode",
        projectRoot: directory,
        input: platformInput
      });
      let context = blocksText(response);
      if (!context) {
        return;
      }
      if (transcriptHasAssistantMessage(messages)) {
        context = stripFirstReplyNotice(context);
      }
      prependEphemeralText(messages, context);
    }
  };
};
export {
  session_start_default as default
};
