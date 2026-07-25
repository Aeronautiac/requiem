// armonia's implementation of amane's host seam.
//
// This is the ONE host that does not speak yagami's protocol: armonia hosts a lawliet engine
// in-process over Tauri IPC, with no server in between. That is the point of it — a dev tool
// that drives the engine directly, switches perspectives, and rewinds.
//
// Every other host, INCLUDING a real packaged Tauri client, talks to yagami over the
// websocket protocol instead. So this file is not "the desktop host"; it is "the no-server
// host", and it exists to be the odd one out.
//
// Because there is no server, it SPOOFS the parts of the contract amane depends on:
//   - seq is synthesised locally. One initiator, one delivery path, so seqs are contiguous
//     and never gap and the Sequencer applies every batch immediately.
//   - the engine's ExecutionResult is repackaged into the server's Batch envelope, so amane
//     sees the shape it would get off a socket.
//   - there is no push stream: nothing else can act on this engine, so the only batches are
//     the ones this client's own inputs cause.
//   - `platform` is absent — there are no games to create or destroy, only the one engine —
//     so amane's platform-administration UI does not render. Context injection, not a check.
//   - there are no keys, so `connect` accepts any credential and every control is Denied.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import type {
  AppExecution,
  Batch,
  ServerInput,
  ServerOutput,
} from "amane/bindings";
import type {
  GameConnection,
  HostContext,
  Toast,
} from "amane/lib/protocol";

function sendActionIpc(action: unknown): Promise<AppExecution> {
  return invoke("send_action", { action });
}

// Repackage the direct-IPC result as the server's Batch. The engine returns commands inside
// the ActionContext on BOTH result arms; the server hoists them into the batch and leaves
// only the response in the reply, so do the same.
function toBatch(input: ServerInput, execution: AppExecution): Batch {
  const { exec_result } = execution;

  if (exec_result === "Crashed") {
    return { commands: [], response: { input, output: { Action: "Crashed" } } };
  }

  const result = exec_result.Standard;
  if ("Err" in result) {
    const [error, context] = result.Err;
    return {
      commands: context.commands,
      response: { input, output: { Action: { Err: error } } },
    };
  }

  const [response, context] = result.Ok;
  return {
    commands: context.commands,
    response: { input, output: { Action: { Ok: response } } },
  };
}

function connectInProcess(): GameConnection {
  let seq = 0;
  let handler: ((batch: ServerOutput) => void) | undefined;

  const emit = (batch: Batch) =>
    handler?.({ seq_num: ++seq, data: { Batch: batch } });

  return {
    send(input: ServerInput): void {
      if ("Control" in input) {
        // Controls are server concepts — teardown, key management. There is neither here, so
        // answer the way the wire would rather than dropping it and leaving a caller waiting
        // on a reply forever.
        emit({ commands: [], response: { input, output: { Control: "Denied" } } });
        return;
      }

      // The IPC call is async but `send` is not: the reply arrives on the batch stream,
      // exactly as it would from a socket, so nothing awaits it here.
      void sendActionIpc(input.Action).then((execution) =>
        emit(toBatch(input, execution)),
      );
    },

    onBatch(next: (batch: ServerOutput) => void): () => void {
      handler = next;
      return () => {
        handler = undefined;
      };
    },

    close(): void {
      // The engine outlives any "connection" here; there is nothing to tear down. Detaching
      // the handler is what actually stops batches reaching a discarded client.
      handler = undefined;
    },
  };
}

export function createTauriHost(): HostContext {
  return {
    canQuit: true,

    // One engine, already running, and no credential that could select anything else — so
    // there is nothing for a join screen to ask. amane joins this at startup and goes
    // straight to the game. The values are placeholders; `connect` ignores them.
    implicitGame: { gameId: 0, key: "" },

    // No keys and no game registry, so the credential is ignored and joining is immediate.
    async connect(): Promise<GameConnection> {
      return connectInProcess();
    },

    async notify({ title, body }: Toast): Promise<void> {
      // Permission is checked lazily on first use (the OS caches the grant), so nothing has
      // to run at startup. Any failure — denied prompt, unavailable service — is swallowed:
      // a missing toast must never break state application.
      try {
        let granted = await isPermissionGranted();
        if (!granted) granted = (await requestPermission()) === "granted";
        if (granted) sendNotification({ title, body });
      } catch {
        // no-op: notifications are best-effort UX
      }
    },

    quit() {
      getCurrentWindow().close();
    },
  };
}
