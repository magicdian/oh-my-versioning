// phronia-shim: opencode@v1
// shim-version: 2610.27.8
// build-id: ec26ef124d33
// content-hash: 2f0d7814e28df9a6895b7703f0eeb1292b68b231e8a312439e31f2a61b73af1e
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
function findUserTextPart(parts) {
  if (!Array.isArray(parts)) return void 0;
  return parts.find(
    (part) => part?.type === "text" && part.synthetic !== true && part.text !== void 0
  );
}
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
function latestUserPromptText(messages) {
  const index = findLatestUserMessageIndex(messages);
  if (index < 0) return "";
  const part = findUserTextPart(messages[index].parts);
  return typeof part?.text === "string" ? part.text : "";
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

// packages/cli/src/templates/opencode/plugins/inject-workflow-state.js
var inject_workflow_state_default = async ({ directory }) => {
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
      const prompt = latestUserPromptText(messages);
      const response = runContext({
        event: "user-prompt",
        platform: "opencode",
        projectRoot: directory,
        input: { ...platformInput, prompt }
      });
      const breadcrumb = blocksText(response);
      if (!breadcrumb) {
        return;
      }
      prependEphemeralText(messages, breadcrumb);
    }
  };
};
export {
  inject_workflow_state_default as default
};
