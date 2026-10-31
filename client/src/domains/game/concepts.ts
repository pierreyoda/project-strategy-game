import type { IconName } from "../../shared/ui/icons";
import { type Resource, RESOURCE_NAMES } from "./resources";

/**
 * What the game can explain about itself, one card at a time.
 *
 * A concept is a word the interface uses - population, electricity, a fission reactor - with what it
 * means in a line or two, where the other concepts in that line are marked. The concept cards of
 * `shared/ui` draw from this; nothing here knows how they look. Client only: the simulation has no
 * use for explanations, and the wording is the interface's to change.
 */
/**
 * A concept the interface can explain, and the words of that explanation that lead to other concepts.
 *
 * This is the shape the concept cards of `ui/concepts` draw from; `GameConcept` below is what the
 * concepts of this game are.
 */
export type ConceptId = string;

/** A run of a concept's summary: plain words, or another concept that opens from them. */
export type ConceptSegment = { text: string } | { concept: ConceptId; text?: string };

export type Concept = {
  id: ConceptId;
  title: string;
  /** What kind of thing it is - "Resource", "Building"... - shown beside the title. */
  kind?: string;
  icon?: IconName;
  /** What it is, in a line or two, with the concepts it leads to marked. */
  summary: readonly ConceptSegment[];
};

/** How a card finds the concepts its links name: a registry, a map, a function - whatever the caller has. */
export type ConceptLookup = (id: ConceptId) => Concept | undefined;

/**
 * Parse a summary written as prose with `[[id]]` or `[[id|shown words]]` marking the concepts in it.
 *
 * Writing "Burns [[uranium]] into electricity" beats building segment arrays by hand, and keeps the
 * sentence readable where it is defined.
 */
export const conceptText = (markup: string): ConceptSegment[] => {
  const segments: ConceptSegment[] = [];
  // `[` excluded from ids and shown words, for a link left unclosed not to swallow the next one
  const pattern = /\[\[([^[\]|]+)(?:\|([^[\]]+))?\]\]/g;
  let last = 0;
  for (const match of markup.matchAll(pattern)) {
    if (match.index > last) {
      segments.push({ text: markup.slice(last, match.index) });
    }
    const [, concept, text] = match;
    segments.push(text ? { concept, text } : { concept });
    last = match.index + match[0].length;
  }
  if (last < markup.length) {
    segments.push({ text: markup.slice(last) });
  }
  return segments;
};

export type GameConceptKind = "Resource" | "Building" | "Mechanic" | "People" | "World";

export type GameConcept = Concept & {
  kind: GameConceptKind;
  /** The resource this concept is, for a card to show the figures of. */
  resource?: Resource;
};

/** Every resource is a concept too: the toolbar's figures each open their own card. */
export type GameConceptId =
  | Resource
  | "population"
  | "housing"
  | "morale"
  | "stability"
  | "shortage"
  | "supply"
  | "upkeep"
  | "research"
  | "settlement"
  | "fission-reactor"
  | "deposit"
  | "fog-of-war"
  | "nation";

const resource = (id: Resource, summary: string): GameConcept => ({
  id,
  title: RESOURCE_NAMES[id],
  kind: "Resource",
  icon: id,
  resource: id,
  summary: conceptText(summary),
});

export const GAME_CONCEPTS: Record<GameConceptId, GameConcept> = {
  population: {
    id: "population",
    title: "Population",
    kind: "People",
    icon: "population",
    summary: conceptText(
      "Grows with a [[food]] surplus and free [[housing]]. Drives [[electricity]] demand and [[morale]].",
    ),
  },
  housing: {
    id: "housing",
    title: "Housing",
    kind: "Mechanic",
    summary: conceptText(
      "Room for the [[population]] of a [[settlement]]. Full, it stops growing; short, [[morale]] drops.",
    ),
  },
  morale: {
    id: "morale",
    title: "Morale",
    kind: "People",
    summary: conceptText("How the [[population]] feels about its lot. Low for long, it turns on [[stability]]."),
  },
  stability: {
    id: "stability",
    title: "Stability",
    kind: "People",
    summary: conceptText(
      "Whether a [[nation]] holds together. Fed by [[morale]], spent by [[shortage|shortages]] and war.",
    ),
  },
  credits: resource(
    "credits",
    "What everything is paid with: buildings, units, and the [[upkeep]] of both, every turn.",
  ),
  food: resource("food", "Feeds the [[population]]; a surplus grows it, a [[shortage]] starves it."),
  water: resource("water", "Drawn for the [[population]] and for farming [[food]]."),
  electricity: resource(
    "electricity",
    "Powers [[settlement|settlements]] and [[research]]. A [[shortage]] halts production.",
  ),
  metals: resource("metals", "Mined from [[deposit|deposits]]; what buildings and units are made of."),
  oil: resource(
    "oil",
    "Pumped from [[deposit|deposits]]; fuels vehicles and the plants that burn it for [[electricity]].",
  ),
  uranium: resource(
    "uranium",
    "Mined from [[deposit|deposits]]; the only fuel a [[fission-reactor]] takes. Run dry and every reactor goes dark the turn after.",
  ),
  shortage: {
    id: "shortage",
    title: "Shortage",
    kind: "Mechanic",
    icon: "warning",
    summary: conceptText(
      "A turn that ends with less than nothing of a resource. Whatever it fed stops: production, growth, reactors.",
    ),
  },
  supply: {
    id: "supply",
    title: "Supply",
    kind: "Mechanic",
    summary: conceptText("The line from a [[settlement]] to what it feeds. Cut, buildings go cold and units run down."),
  },
  upkeep: {
    id: "upkeep",
    title: "Upkeep",
    kind: "Mechanic",
    summary: conceptText(
      "What a building or unit costs every turn to maintain in optimal condition, taken before anything else.",
    ),
  },
  research: {
    id: "research",
    title: "Research",
    kind: "Mechanic",
    icon: "research",
    summary: conceptText("Turns [[electricity]] and people into new buildings and units, one project at a time."),
  },
  settlement: {
    id: "settlement",
    title: "Settlement",
    kind: "World",
    icon: "settlement",
    summary: conceptText(
      "Where a [[nation]]'s [[population]] lives and works, from outpost to capital. Draws [[electricity]] every turn.",
    ),
  },
  "fission-reactor": {
    id: "fission-reactor",
    title: "Fission reactor",
    kind: "Building",
    summary: conceptText(
      "Burns [[uranium]] into electricity each turn. Costs credits in [[upkeep]]; goes cold the turn its [[supply]] is cut.",
    ),
  },
  deposit: {
    id: "deposit",
    title: "Deposit",
    kind: "World",
    summary: conceptText(
      "A tile's hoard of [[metals]] or [[uranium]], mined down a little every turn until it is gone.",
    ),
  },
  "fog-of-war": {
    id: "fog-of-war",
    title: "Fog of war",
    kind: "Mechanic",
    icon: "map",
    summary: conceptText(
      "What a [[nation]] cannot see right now. Explored once, a tile stays on the map as it was last seen.",
    ),
  },
  nation: {
    id: "nation",
    title: "Nation",
    kind: "World",
    summary: conceptText(
      "A player, human or not: its [[settlement|settlements]], units, stockpiles and [[stability]].",
    ),
  },
};

/** The lookup the concept cards take, for anything in the game to open a chain. */
export const lookupGameConcept = (id: ConceptId): GameConcept | undefined => GAME_CONCEPTS[id as GameConceptId];
