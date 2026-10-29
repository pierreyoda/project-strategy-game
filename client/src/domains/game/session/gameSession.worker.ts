import type {
  GameEvent,
  GameSessionRequest,
  GameSessionResponse,
  ProjectionMeta,
  SettlementsBuffers,
  TurnPhase,
  UnitsBuffers,
  WorldDeltaData,
  WorldProjection,
  WorldSnapshotData,
} from "./protocol";

// Imported lazily, like in `planet.worker.ts`: instantiating the WASM bridge relies on top-level
// await, during which a module worker already receives messages, before `onmessage` would be set.
const bridge = import("../../../../../wasm-pkg");

type Bridge = Awaited<typeof bridge>;
// not `InstanceType`: the bridge's constructor is private, a session only comes from `newGame`
type Session = ReturnType<Bridge["GameSession"]["newGame"]>;

/**
 * The session lives here, and only here.
 *
 * It is long-lived and stateful: unlike the planet generator, it is never terminated to interrupt
 * it, because its state *is* the game. Cancelling a turn is a message read between two steps.
 */
let session: Session | null = null;

/** Set the moment a cancellation arrives, read between two steps of the turn loop. */
let cancelRequested = false;

/** Requests are handled one after another, so that two of them never touch the session at once. */
let queue: Promise<void> = Promise.resolve();

const respond = (response: GameSessionResponse, transfer: Transferable[] = []) => postMessage(response, { transfer });

/** Let the worker dispatch the messages waiting in its inbox, a cancellation among them. */
const yieldToMessages = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

const requireSession = (): Session => {
  if (!session) {
    throw new Error("no game session was started.");
  }
  return session;
};

const parseMeta = (json: string): ProjectionMeta => JSON.parse(json) as ProjectionMeta;

/** Every buffer of a projection, so that they are moved to the main thread rather than copied. */
const buffersOf = (projection: WorldProjection): Transferable[] => {
  const entities = (units: UnitsBuffers, settlements: SettlementsBuffers) => [
    units.ids.buffer,
    units.tiles.buffer,
    units.owners.buffer,
    units.health.buffer,
    settlements.ids.buffer,
    settlements.tiles.buffer,
    settlements.owners.buffer,
  ];
  if (projection.kind === "full") {
    const { snapshot } = projection;
    return [
      snapshot.terrain.buffer,
      snapshot.elevations.buffer,
      snapshot.visibility.buffer,
      snapshot.tileOwner.buffer,
      snapshot.tileFlags.buffer,
      ...entities(snapshot.units, snapshot.settlements),
    ];
  }
  const { delta } = projection;
  return [
    delta.tileIndices.buffer,
    delta.elevations.buffer,
    delta.visibility.buffer,
    delta.tileOwner.buffer,
    delta.tileFlags.buffer,
    delta.unitsRemoved.buffer,
    delta.settlementsRemoved.buffer,
    ...entities(delta.units, delta.settlements),
  ];
};

const readSnapshot = (view: {
  meta: string;
  columns: number;
  rows: number;
  terrain: Uint8Array;
  elevations: Int16Array;
  visibility: Uint8Array;
  tileOwner: Uint16Array;
  tileFlags: Uint8Array;
  unitIds: Uint32Array;
  unitTiles: Uint32Array;
  unitOwners: Uint16Array;
  unitHealth: Uint16Array;
  settlementIds: Uint32Array;
  settlementTiles: Uint32Array;
  settlementOwners: Uint16Array;
}): WorldSnapshotData => ({
  // every getter copies out of the WASM memory: read each one exactly once
  meta: parseMeta(view.meta),
  columns: view.columns,
  rows: view.rows,
  terrain: view.terrain,
  elevations: view.elevations,
  visibility: view.visibility,
  tileOwner: view.tileOwner,
  tileFlags: view.tileFlags,
  units: { ids: view.unitIds, tiles: view.unitTiles, owners: view.unitOwners, health: view.unitHealth },
  settlements: { ids: view.settlementIds, tiles: view.settlementTiles, owners: view.settlementOwners },
});

const readDelta = (view: {
  meta: string;
  tileIndices: Uint32Array;
  elevations: Int16Array;
  visibility: Uint8Array;
  tileOwner: Uint16Array;
  tileFlags: Uint8Array;
  unitIds: Uint32Array;
  unitTiles: Uint32Array;
  unitOwners: Uint16Array;
  unitHealth: Uint16Array;
  unitsRemoved: Uint32Array;
  settlementIds: Uint32Array;
  settlementTiles: Uint32Array;
  settlementOwners: Uint16Array;
  settlementsRemoved: Uint32Array;
}): WorldDeltaData => ({
  meta: parseMeta(view.meta),
  tileIndices: view.tileIndices,
  elevations: view.elevations,
  visibility: view.visibility,
  tileOwner: view.tileOwner,
  tileFlags: view.tileFlags,
  units: { ids: view.unitIds, tiles: view.unitTiles, owners: view.unitOwners, health: view.unitHealth },
  unitsRemoved: view.unitsRemoved,
  settlements: { ids: view.settlementIds, tiles: view.settlementTiles, owners: view.settlementOwners },
  settlementsRemoved: view.settlementsRemoved,
});

/** Hand over everything the simulation buffered, so the UI log never lags behind the turn. */
const flushEvents = (current: Session) => {
  const events = JSON.parse(current.drainEvents()) as GameEvent[];
  if (events.length > 0) {
    respond({ type: "events", events });
  }
};

/**
 * Resolve a turn, one bounded step at a time.
 *
 * This loop *is* the notification channel: the simulation never calls back into JavaScript, it just
 * returns after each slice of work and lets this decide what to do next - post progress, flush
 * events, or stop because a cancellation came in.
 */
const resolveTurn = async (id: number) => {
  const current = requireSession();
  cancelRequested = false;
  current.beginTurn();

  for (; ;) {
    const step = current.step();
    let done: boolean;
    let idle: boolean;
    let phase: TurnPhase;
    let percent: number;
    let turn: number;
    let version: number;
    try {
      ({ done, idle, percent, turn, version } = step);
      phase = step.phase as TurnPhase;
    } finally {
      step.free();
    }

    flushEvents(current);

    if (idle) {
      respond({ type: "error", id, message: "the session stopped resolving mid-turn." });
      return;
    }
    if (done) {
      respond({ type: "turnComplete", id, turn, version });
      return;
    }
    respond({ type: "progress", phase, percent });

    if (cancelRequested) {
      cancelRequested = false;
      // the world is left between two phases, which is a consistent place to be: sending `endTurn`
      // again picks the resolution up where it stopped
      respond({ type: "turnCancelled", id });
      return;
    }
    await yieldToMessages();
  }
};

const handle = async (request: GameSessionRequest) => {
  switch (request.type) {
    case "start": {
      const { GameSession } = await bridge;
      session?.free();
      session = GameSession.newGame(JSON.stringify(request.settings));
      flushEvents(session);
      respond({
        type: "ready",
        id: request.id,
        localPlayer: session.localPlayer,
        turn: session.turn,
        version: session.version,
      });
      return;
    }
    case "submit": {
      const current = requireSession();
      const rejection = current.submit(JSON.stringify(request.command));
      flushEvents(current);
      if (rejection === undefined) {
        respond({ type: "accepted", id: request.id, version: current.version });
      } else {
        respond({ type: "rejected", id: request.id, rejection: JSON.parse(rejection) });
      }
      return;
    }
    case "endTurn":
      await resolveTurn(request.id);
      return;
    case "snapshot": {
      const view = requireSession().snapshot(request.viewer);
      let projection: WorldProjection;
      try {
        projection = { kind: "full", snapshot: readSnapshot(view) };
      } finally {
        view.free();
      }
      respond({ type: "projection", id: request.id, projection }, buffersOf(projection));
      return;
    }
    case "project": {
      const view = requireSession().project(request.viewer, request.since);
      let projection: WorldProjection;
      try {
        const full = view.takeFull();
        if (full) {
          try {
            projection = { kind: "full", snapshot: readSnapshot(full) };
          } finally {
            full.free();
          }
        } else {
          const delta = view.takeDelta();
          if (!delta) {
            throw new Error("the projection held neither a snapshot nor a delta.");
          }
          try {
            projection = { kind: "delta", delta: readDelta(delta) };
          } finally {
            delta.free();
          }
        }
      } finally {
        view.free();
      }
      respond({ type: "projection", id: request.id, projection }, buffersOf(projection));
      return;
    }
  }
};

onmessage = ({ data: request }: MessageEvent<GameSessionRequest>) => {
  // read out of band: queueing it would put it behind the very turn it is meant to stop
  if (request.type === "cancelTurn") {
    cancelRequested = true;
    return;
  }
  queue = queue.then(() =>
    handle(request).catch((error: unknown) =>
      respond({
        type: "error",
        id: request.id,
        message: error instanceof Error ? error.message : String(error),
      }),
    ),
  );
};
