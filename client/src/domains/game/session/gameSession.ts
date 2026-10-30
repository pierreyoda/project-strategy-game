import type {
  CommandRejection,
  GameCommand,
  GameEvent,
  GameSessionRequest,
  GameSessionResponse,
  NewGameSettings,
  ReachableData,
  TurnPhase,
  WorldProjection,
} from "./protocol";

/** What a running session tells its listeners, as it happens. */
export type GameSessionNotification =
  | { type: "progress"; phase: TurnPhase; percent: number }
  | { type: "events"; events: readonly GameEvent[] }
  | { type: "error"; message: string };

export type GameSessionListener = (notification: GameSessionNotification) => void;

/** How a turn ended. */
export type TurnOutcome =
  | { type: "complete"; turn: number; version: number }
  /** Stopped between two phases. Ending the turn again resumes where it stopped. */
  | { type: "cancelled" };

export type GameSessionHandle = {
  /** Start a new game, replacing the one in progress if any. */
  start: (settings: NewGameSettings) => Promise<{ localPlayer: number; turn: number; version: number }>;
  /** Give an order. Resolves with `null` when it was taken, or with why it was refused. */
  submit: (command: GameCommand) => Promise<CommandRejection | null>;
  /** Resolve a turn, reporting its progress to the listeners along the way. */
  endTurn: () => Promise<TurnOutcome>;
  /** Stop the turn in progress between two phases. */
  cancelTurn: () => void;
  /** Everything the viewer can see, from scratch. */
  snapshot: (viewer: number) => Promise<WorldProjection>;
  /** What the viewer needs to catch up with `since`: a delta, or a whole new snapshot. */
  project: (viewer: number, since: number) => Promise<WorldProjection>;
  /** Where the viewer's unit can get to this turn, `null` for a unit that is not the viewer's. */
  reachable: (viewer: number, unit: number) => Promise<ReachableData | null>;
  /** Listen to progress, events and errors. Returns the unsubscriber. */
  subscribe: (listener: GameSessionListener) => () => void;
  /** Release the worker. The session is gone: this is not how a turn is cancelled. */
  dispose: () => void;
};

/**
 * Drives the simulation in a Web Worker, so as not to block the UI.
 *
 * Same shape as `createPlanetGenerator`, with two differences that matter:
 *
 * - the worker is **long-lived and stateful**, and must never be terminated to interrupt it: the
 *   planet generator can be killed because a half-generated planet is worth nothing, while this
 *   worker holds the game itself. Cancelling a turn is a message, read between two steps;
 * - besides its promises it exposes a **subscription**, because a turn reports progress and events
 *   while it resolves rather than only at the end.
 */
export const createGameSession = (): GameSessionHandle => {
  const worker = new Worker(new URL("./gameSession.worker.ts", import.meta.url), { type: "module" });
  const listeners = new Set<GameSessionListener>();
  const pending = new Map<
    number,
    { resolve: (response: GameSessionResponse) => void; reject: (error: Error) => void }
  >();
  let nextRequestId = 1;
  let disposed = false;

  const notify = (notification: GameSessionNotification) => {
    for (const listener of listeners) {
      listener(notification);
    }
  };

  worker.onmessage = ({ data: response }: MessageEvent<GameSessionResponse>) => {
    if (response.type === "progress" || response.type === "events") {
      notify(response);
      return;
    }
    if (response.type === "error") {
      const settled = response.id === null ? undefined : pending.get(response.id);
      if (response.id !== null) {
        pending.delete(response.id);
      }
      // an error with no request to blame is reported to the listeners instead
      if (settled) {
        settled.reject(new Error(response.message));
      } else {
        notify({ type: "error", message: response.message });
      }
      return;
    }
    const settled = pending.get(response.id);
    if (settled) {
      pending.delete(response.id);
      settled.resolve(response);
    }
  };

  // the worker itself failed, for instance to load the WASM bridge: everything in flight is lost
  worker.onerror = (event) => {
    event.preventDefault();
    const error = new Error(`The game session failed: ${event.message}`);
    for (const settled of pending.values()) {
      settled.reject(error);
    }
    pending.clear();
    notify({ type: "error", message: error.message });
  };

  const request = (build: (id: number) => GameSessionRequest): Promise<GameSessionResponse> => {
    if (disposed) {
      return Promise.reject(new Error("the game session was disposed."));
    }
    const id = nextRequestId++;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      worker.postMessage(build(id));
    });
  };

  const unexpected = (response: GameSessionResponse): never => {
    throw new Error(`unexpected response from the game session: ${response.type}`);
  };

  return {
    start: async (settings) => {
      const response = await request((id) => ({ type: "start", id, settings }));
      return response.type === "ready" ? response : unexpected(response);
    },
    submit: async (command) => {
      const response = await request((id) => ({ type: "submit", id, command }));
      if (response.type === "accepted") {
        return null;
      }
      return response.type === "rejected" ? response.rejection : unexpected(response);
    },
    endTurn: async () => {
      const response = await request((id) => ({ type: "endTurn", id }));
      if (response.type === "turnComplete") {
        return { type: "complete", turn: response.turn, version: response.version };
      }
      return response.type === "turnCancelled" ? { type: "cancelled" } : unexpected(response);
    },
    cancelTurn: () => {
      if (!disposed) {
        worker.postMessage({ type: "cancelTurn" } satisfies GameSessionRequest);
      }
    },
    snapshot: async (viewer) => {
      const response = await request((id) => ({ type: "snapshot", id, viewer }));
      return response.type === "projection" ? response.projection : unexpected(response);
    },
    project: async (viewer, since) => {
      const response = await request((id) => ({ type: "project", id, viewer, since }));
      return response.type === "projection" ? response.projection : unexpected(response);
    },
    reachable: async (viewer, unit) => {
      const response = await request((id) => ({ type: "reachable", id, viewer, unit }));
      return response.type === "reachable" ? response.reach : unexpected(response);
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispose: () => {
      disposed = true;
      for (const settled of pending.values()) {
        settled.reject(new Error("the game session was disposed."));
      }
      pending.clear();
      listeners.clear();
      worker.terminate();
    },
  };
};
