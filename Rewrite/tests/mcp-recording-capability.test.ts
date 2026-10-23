import { expect, test } from "bun:test";
import assert from "node:assert/strict";
import { createServer, type Server } from "node:net";
import { randomUUID } from "node:crypto";
import { mkdtempSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { Client } from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";
const root = resolve(import.meta.dir, "..");
const receiver = join(
  root,
  `native/receiver/target/debug/thumble-rewrite-receiver${process.platform === "win32" ? ".exe" : ""}`,
);
const cli = join(root, "apps/cli/src/main.ts");
const profile = "e1000000-0000-0000-0000-000000000001";
const sourceHash = "b".repeat(64);
const calls = [
  {
    name: "receiver_activate",
    arguments: { expectedRevision: "0", profileId: profile },
  },
  {
    name: "receiver_activate_saved",
    arguments: {
      expectedReceiverRevision: "0",
      expectedConfigurationRevision: "1",
      expectedSourceHash: sourceHash,
      profileId: profile,
    },
  },
  {
    name: "receiver_sample",
    arguments: {
      expectedGeneration: "1",
      sequence: "0",
      state: {
        buttons: [],
        leftStickX: 0,
        leftStickY: 0,
        rightStickX: 0,
        rightStickY: 0,
        leftTrigger: 0,
        rightTrigger: 0,
      },
    },
  },
];
async function listening(server: Server): Promise<number> {
  await new Promise<void>((r) => server.listen(0, "127.0.0.1", r));
  const address = server.address();
  assert(address && typeof address !== "string");
  return address.port;
}
async function close(server: Server) {
  await new Promise<void>((r) => server.close(() => r()));
}
/** Capability-authenticated service double; never constructs any output owner. */
async function fixture(
  backend: "recording" | "macos-hid",
  replaceAfterStatus = false,
) {
  const directory = realpathSync(
    mkdtempSync(join(tmpdir(), "thumble-mcp-capability-")),
  );
  const instanceId = randomUUID();
  const seen: string[] = [];
  const replacementSeen: string[] = [];
  let active = false;
  let replaced = false;
  const snapshot = () => ({
    backend,
    inputEnabled: false,
    active,
    generation: active ? "1" : "0",
    revision: active ? "1" : "0",
    profileId: active ? profile : null,
    sequence: null,
    reportHex: "0014000000000000000000000000000000000000",
    lastReleaseReason: "startup",
  });
  const status = () => ({
    service: {
      instanceId,
      pid: process.pid,
      configuration: "ephemeral-runtime-only",
    },
    receiver: { pid: process.pid, snapshot: snapshot() },
    permissions: "not-evaluated",
    deviceRecognition: "not-evaluated",
    phone: "not-evaluated",
  });
  const endpoint = (port: number, token = "a".repeat(64)) => ({
    version: 1,
    instanceId,
    servicePid: process.pid,
    receiverPid: process.pid,
    port,
    token,
    recordingLog: join(directory, "not-created.jsonl"),
  });
  const save = (port: number, token?: string) =>
    writeFileSync(
      join(directory, "endpoint.json"),
      JSON.stringify(endpoint(port, token)),
      { mode: 0o600 },
    );
  const replacement = createServer((socket) =>
    socket.on("data", (chunk) => {
      const request = JSON.parse(chunk.toString());
      replacementSeen.push(request.body.command);
      socket.end(
        JSON.stringify({
          version: 1,
          requestId: request.requestId,
          ok: false,
          error: "replacement_must_not_receive_mutation",
        }) + "\n",
      );
    }),
  );
  const replacementPort = await listening(replacement);
  const server = createServer((socket) => {
    let bytes = "";
    socket.on("data", (chunk) => {
      bytes += chunk.toString();
      assert(bytes.length <= 65536);
      if (!bytes.includes("\n")) return;
      const request = JSON.parse(bytes.trim());
      assert.equal(request.token, "a".repeat(64));
      const command = request.body.command;
      seen.push(command);
      if (command === "status" && replaceAfterStatus && !replaced) {
        replaced = true;
        save(replacementPort, "c".repeat(64));
      }
      let result: object;
      if (command === "status") result = status();
      else if (command === "release") {
        active = false;
        result = status();
      } else if (backend !== "recording") {
        socket.end(
          JSON.stringify({
            version: 1,
            requestId: request.requestId,
            ok: false,
            error: "physical_mutation_dispatched",
          }) + "\n",
        );
        return;
      } else if (command === "activate-saved") {
        active = true;
        result = {
          kind: "saved-runtime",
          serviceInstanceId: instanceId,
          servicePid: process.pid,
          receiverPid: process.pid,
          snapshot: snapshot(),
          savedBinding: {
            configurationRevision: "1",
            sourceHash,
            profileId: profile,
            generation: "1",
            revision: "1",
          },
        };
      } else {
        active = true;
        result = status();
      }
      socket.end(
        JSON.stringify({
          version: 1,
          requestId: request.requestId,
          ok: true,
          result,
        }) + "\n",
      );
    });
  });
  const port = await listening(server);
  save(port);
  const transport = new StdioClientTransport({
    command: process.execPath,
    args: [
      cli,
      "mcp",
      "serve",
      "--runtime-dir",
      directory,
      "--receiver",
      receiver,
      "--allow-recorded-input",
    ],
    stderr: "pipe",
  });
  let stderr = "";
  transport.stderr?.on("data", (chunk) => {
    stderr += chunk.toString();
    assert(stderr.length <= 4096);
  });
  const client = new Client(
    { name: "recording-capability-regression", version: "1" },
    { versionNegotiation: { mode: { pin: "2026-07-28" } } },
  );
  try {
    await client.connect(transport, { timeout: 5000 });
  } catch (error) {
    await client.close();
    await close(server);
    await close(replacement);
    rmSync(directory, { recursive: true, force: true });
    throw error;
  }
  return {
    client,
    seen,
    replacementSeen,
    async stop() {
      await client.close();
      await close(server);
      await close(replacement);
      rmSync(directory, { recursive: true, force: true });
      expect(stderr).toBe("");
    },
  };
}
test("all recording-only MCP tools refuse physical status before dispatch; emergency release remains available", async () => {
  const f = await fixture("macos-hid");
  try {
    for (const call of calls)
      expect(await f.client.callTool(call)).toMatchObject({
        isError: true,
        structuredContent: { ok: false, error: "recording_receiver_required" },
      });
    expect(f.seen).toEqual(["status", "status", "status"]);
    expect(
      await f.client.callTool({ name: "receiver_release", arguments: {} }),
    ).toMatchObject({ structuredContent: { ok: true } });
    expect(f.seen).toEqual(["status", "status", "status", "release"]);
  } finally {
    await f.stop();
  }
}, 15000);
test("recording MCP activation, saved activation and sample still dispatch", async () => {
  const f = await fixture("recording");
  try {
    for (const call of calls)
      expect(await f.client.callTool(call)).toMatchObject({
        structuredContent: { ok: true },
      });
    expect(f.seen).toEqual([
      "status",
      "activate",
      "status",
      "activate-saved",
      "status",
      "sample",
    ]);
  } finally {
    await f.stop();
  }
}, 15000);
test("endpoint replacement after status cannot retarget a guarded command to another owner", async () => {
  const f = await fixture("recording", true);
  try {
    expect(await f.client.callTool(calls[0]!)).toMatchObject({
      structuredContent: { ok: true },
    });
    expect(f.seen).toEqual(["status", "activate"]);
    expect(f.replacementSeen).toEqual([]);
  } finally {
    await f.stop();
  }
}, 15000);
