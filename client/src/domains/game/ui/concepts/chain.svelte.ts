import type { Attachment } from "svelte/attachments";

import { type ConceptId, type GameConcept, lookupGameConcept } from "../../concepts";

/**
 * The timings of a chain of concept cards, in milliseconds.
 *
 * `appear` is the ordinary tooltip delay. `dwell` is how long the pointer has to rest on a card's owner
 * before the card may be entered; the bar on its side shows it passing. `grace` is how long a card
 * survives the pointer leaving it, so that crossing a gap towards its child costs nothing.
 */
export const CONCEPT_TIMING = { appear: 300, dwell: 600, grace: 250 } as const;

/** How deep a chain goes. The last card's links are plain words. */
export const MAX_CONCEPT_DEPTH = 4;

export type ConceptLevel = {
  concept: GameConcept;
  /** The element the card hangs from: the owner for the first card, a link in the parent for the others. */
  anchor: HTMLElement;
  /** Share of the dwell time elapsed, 1 once the card is open. */
  dwell: number;
  /** Whether the pointer may come in: the dwell is done, or the card was opened by keyboard. */
  open: boolean;
  /** Opened by keyboard focus: no dwell, and the hints speak of keys rather than of the pointer. */
  byKeyboard: boolean;
};

/**
 * The chain of concept cards currently shown, as the state behind `ConceptChain`.
 *
 * One chain per interface, reading the game's concepts: everything that owns a concept reports to it through `conceptOwner`, the
 * cards report the pointer entering and leaving them, and the links inside the cards report being
 * hovered. The chain decides what that means, so the rules written on the UI kit's board live here
 * and nowhere else.
 */
export type ConceptChain = ReturnType<typeof createConceptChain>;

export const createConceptChain = () => {
  const levels = $state<ConceptLevel[]>([]);

  let appearTimer: ReturnType<typeof setTimeout> | undefined;
  let closeTimer: ReturnType<typeof setTimeout> | undefined;
  let dwellFrame: number | undefined;
  /** The depth a pending close starts from: cards at this depth and deeper go when the grace runs out. */
  let closeFrom = Infinity;

  const cancelAppear = () => {
    clearTimeout(appearTimer);
    appearTimer = undefined;
  };
  const cancelClose = () => {
    clearTimeout(closeTimer);
    closeTimer = undefined;
    closeFrom = Infinity;
  };
  const stopDwell = () => {
    if (dwellFrame !== undefined) {
      cancelAnimationFrame(dwellFrame);
      dwellFrame = undefined;
    }
  };

  /** Drop the cards at `depth` and deeper. */
  const truncate = (depth: number) => {
    if (levels.length > depth) {
      stopDwell();
      levels.splice(depth);
    }
  };

  const scheduleClose = (depth: number) => {
    cancelClose();
    closeFrom = depth;
    closeTimer = setTimeout(() => {
      truncate(depth);
      closeFrom = Infinity;
    }, CONCEPT_TIMING.grace);
  };

  /** Fill the dwell bar of the card at `depth` over the dwell time, then open it. */
  const runDwell = (depth: number) => {
    const start = performance.now();
    const tick = (now: number) => {
      const level = levels[depth];
      if (!level) {
        return;
      }
      level.dwell = Math.min(1, (now - start) / CONCEPT_TIMING.dwell);
      if (level.dwell < 1) {
        dwellFrame = requestAnimationFrame(tick);
      } else {
        level.open = true;
        dwellFrame = undefined;
      }
    };
    dwellFrame = requestAnimationFrame(tick);
  };

  /** Show a card at `depth`, in place of whatever was there and deeper. */
  const startLevel = (depth: number, concept: GameConcept, anchor: HTMLElement, byKeyboard: boolean) => {
    cancelAppear();
    cancelClose();
    truncate(depth);
    levels.push({ concept, anchor, dwell: byKeyboard ? 1 : 0, open: byKeyboard, byKeyboard });
    if (!byKeyboard) {
      runDwell(depth);
    }
  };

  /** Show a card at `depth` once the pointer has rested on its owner for the tooltip delay. */
  const appear = (depth: number, concept: GameConcept, anchor: HTMLElement) => {
    cancelAppear();
    cancelClose();
    appearTimer = setTimeout(() => startLevel(depth, concept, anchor, false), CONCEPT_TIMING.appear);
  };

  /** Open the card of `id` at `depth`, unless that card is already there: then only keep it. */
  const show = (depth: number, id: ConceptId, anchor: HTMLElement, byKeyboard: boolean) => {
    if (depth >= MAX_CONCEPT_DEPTH) {
      return;
    }
    const concept = lookupGameConcept(id);
    if (!concept) {
      return;
    }
    if (levels[depth]?.concept.id === id) {
      cancelClose();
      return;
    }
    if (byKeyboard) {
      startLevel(depth, concept, anchor, true);
    } else {
      appear(depth, concept, anchor);
    }
  };

  return {
    get levels(): readonly ConceptLevel[] {
      return levels;
    },

    /** The pointer rests on something a concept describes. */
    hoverOwner: (id: ConceptId, anchor: HTMLElement) => show(0, id, anchor, false),
    /** The pointer left it: the card, if any, survives the grace, long enough to be entered. */
    leaveOwner: () => {
      cancelAppear();
      if (levels.length > 0) {
        scheduleClose(0);
      }
    },
    /** Keyboard focus reached something a concept describes: its card opens at once. */
    focusOwner: (id: ConceptId, anchor: HTMLElement) => show(0, id, anchor, true),
    blurOwner: () => {
      if (levels.length > 0) {
        scheduleClose(0);
      }
    },

    /** The pointer came into the card at `depth`: it and its ancestors stay. */
    enterCard: (depth: number) => {
      if (depth >= closeFrom) {
        cancelClose();
      }
    },
    /** The pointer left the card at `depth`: it and its descendants go, unless something keeps them. */
    leaveCard: (depth: number) => scheduleClose(depth),

    /** A link in the open card at `depth` is hovered: its concept's card starts appearing beneath it. */
    hoverLink: (depth: number, id: ConceptId, anchor: HTMLElement) => {
      if (levels[depth]?.open) {
        show(depth + 1, id, anchor, false);
      }
    },
    /** The pointer left a link: a child still filling its bar goes, one already open stays for the card to catch. */
    leaveLink: (depth: number) => {
      cancelAppear();
      const child = levels[depth + 1];
      if (child && !child.open) {
        scheduleClose(depth + 1);
      }
    },
    /** A link was activated by keyboard: its card opens at once. */
    openLink: (depth: number, id: ConceptId, anchor: HTMLElement) => {
      if (levels[depth]?.open) {
        show(depth + 1, id, anchor, true);
      }
    },

    /** Close the deepest card, as Escape does. */
    back: () => {
      cancelAppear();
      cancelClose();
      truncate(levels.length - 1);
    },
    closeAll: () => {
      cancelAppear();
      cancelClose();
      truncate(0);
    },
  };
};

/**
 * Make an element the owner of a concept: resting the pointer on it, or focusing it, opens the chain.
 *
 * Usage: `<button {@attach conceptOwner(chain, "electricity")}>128</button>`.
 */
export const conceptOwner =
  (chain: ConceptChain, id: ConceptId): Attachment<HTMLElement> =>
  (node) => {
    const enter = () => chain.hoverOwner(id, node);
    const leave = () => chain.leaveOwner();
    const focus = () => chain.focusOwner(id, node);
    const blur = () => chain.blurOwner();
    node.addEventListener("pointerenter", enter);
    node.addEventListener("pointerleave", leave);
    node.addEventListener("focusin", focus);
    node.addEventListener("focusout", blur);
    return () => {
      node.removeEventListener("pointerenter", enter);
      node.removeEventListener("pointerleave", leave);
      node.removeEventListener("focusin", focus);
      node.removeEventListener("focusout", blur);
    };
  };
