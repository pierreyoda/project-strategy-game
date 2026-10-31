import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { type ConceptId, GAME_CONCEPTS } from "../../concepts";
import { CONCEPT_TIMING, type ConceptChain, conceptOwner, createConceptChain, MAX_CONCEPT_DEPTH } from "./chain.svelte";

const { appear, dwell, grace } = CONCEPT_TIMING;
// the fake timers run animation frames every 16ms: a dwell ends on the first frame past its time
const FRAME = 16;

const element = () => document.createElement("button");
const shownIds = (chain: ConceptChain) => chain.levels.map((level) => level.concept.id);

/** Rest the pointer on the owner of `id` until its card is open. */
const hoverOpen = (chain: ConceptChain, id: ConceptId) => {
  chain.hoverOwner(id, element());
  vi.advanceTimersByTime(appear + dwell + FRAME);
};

/** Open the cards of `ids` by keyboard, each from a link in the previous one. */
const openByKeyboard = (chain: ConceptChain, ...ids: ConceptId[]) => {
  chain.focusOwner(ids[0], element());
  ids.slice(1).forEach((id, depth) => chain.openLink(depth, id, element()));
};

let chain: ConceptChain;

beforeEach(() => {
  vi.useFakeTimers();
  chain = createConceptChain();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("createConceptChain", () => {
  describe("owners under the pointer", () => {
    it("show their card once the pointer has rested on them for the tooltip delay", () => {
      const owner = element();
      chain.hoverOwner("food", owner);
      vi.advanceTimersByTime(appear - 1);
      expect(chain.levels).toHaveLength(0);

      vi.advanceTimersByTime(1);
      expect(shownIds(chain)).toStrictEqual(["food"]);
      expect(chain.levels[0].anchor).toBe(owner);
      expect(chain.levels[0]).toMatchObject({ dwell: 0, open: false, byKeyboard: false });
    });

    it("open their card once its dwell bar is full", () => {
      chain.hoverOwner("food", element());
      vi.advanceTimersByTime(appear + dwell / 2);
      expect(chain.levels[0].dwell).toBeCloseTo(0.5, 1);
      expect(chain.levels[0].open).toBe(false);

      vi.advanceTimersByTime(dwell / 2 - FRAME);
      expect(chain.levels[0].open).toBe(false);

      vi.advanceTimersByTime(2 * FRAME);
      expect(chain.levels[0]).toMatchObject({ dwell: 1, open: true });
    });

    it("show nothing when the pointer leaves them before the delay", () => {
      chain.hoverOwner("food", element());
      vi.advanceTimersByTime(appear - 1);
      chain.leaveOwner();
      vi.runAllTimers();
      expect(chain.levels).toHaveLength(0);
    });

    it("keep their card for the grace once the pointer leaves them", () => {
      hoverOpen(chain, "food");
      chain.leaveOwner();
      vi.advanceTimersByTime(grace - 1);
      expect(shownIds(chain)).toStrictEqual(["food"]);

      vi.advanceTimersByTime(1);
      expect(chain.levels).toHaveLength(0);
    });

    it("keep their card when the pointer crosses into it within the grace", () => {
      hoverOpen(chain, "food");
      chain.leaveOwner();
      vi.advanceTimersByTime(grace - 1);
      chain.enterCard(0);
      vi.runAllTimers();
      expect(shownIds(chain)).toStrictEqual(["food"]);
    });

    it("keep their card as it is when the pointer comes back to them", () => {
      hoverOpen(chain, "food");
      chain.leaveOwner();
      chain.hoverOwner("food", element());
      vi.runAllTimers();
      expect(shownIds(chain)).toStrictEqual(["food"]);
      expect(chain.levels[0].open).toBe(true);
    });

    it("replace the card of another owner once the delay is over", () => {
      hoverOpen(chain, "food");
      chain.hoverOwner("water", element());
      vi.advanceTimersByTime(appear - 1);
      expect(shownIds(chain)).toStrictEqual(["food"]);

      vi.advanceTimersByTime(1);
      expect(shownIds(chain)).toStrictEqual(["water"]);
      expect(chain.levels[0].open).toBe(false);
    });

    it("restart the dwell when replacing a card still filling its bar", () => {
      chain.hoverOwner("food", element());
      vi.advanceTimersByTime(appear + dwell / 2);
      chain.hoverOwner("water", element());
      vi.advanceTimersByTime(appear);
      expect(chain.levels[0]).toMatchObject({ dwell: 0, open: false });

      // the dwell of the card replaced, had it gone on, would have opened this one by now
      vi.advanceTimersByTime(dwell - FRAME);
      expect(chain.levels[0].open).toBe(false);

      vi.advanceTimersByTime(2 * FRAME);
      expect(chain.levels[0].open).toBe(true);
    });

    it("of unknown concepts show nothing", () => {
      chain.hoverOwner("unknown", element());
      chain.focusOwner("unknown", element());
      vi.runAllTimers();
      expect(chain.levels).toHaveLength(0);
    });
  });

  describe("owners under keyboard focus", () => {
    it("open their card at once", () => {
      const owner = element();
      chain.focusOwner("food", owner);
      expect(shownIds(chain)).toStrictEqual(["food"]);
      expect(chain.levels[0].anchor).toBe(owner);
      expect(chain.levels[0]).toMatchObject({ dwell: 1, open: true, byKeyboard: true });
    });

    it("keep their card for the grace once they lose focus", () => {
      chain.focusOwner("food", element());
      chain.blurOwner();
      vi.advanceTimersByTime(grace - 1);
      expect(shownIds(chain)).toStrictEqual(["food"]);

      vi.advanceTimersByTime(1);
      expect(chain.levels).toHaveLength(0);
    });
  });

  describe("links", () => {
    it("show their card beneath them once hovered for the tooltip delay", () => {
      openByKeyboard(chain, "food");
      const link = element();
      chain.hoverLink(0, "population", link);
      vi.advanceTimersByTime(appear - 1);
      expect(shownIds(chain)).toStrictEqual(["food"]);

      vi.advanceTimersByTime(1);
      expect(shownIds(chain)).toStrictEqual(["food", "population"]);
      expect(chain.levels[1].anchor).toBe(link);
      expect(chain.levels[1]).toMatchObject({ dwell: 0, open: false, byKeyboard: false });
    });

    it("open their card at once when activated by keyboard", () => {
      openByKeyboard(chain, "food");
      chain.openLink(0, "population", element());
      expect(shownIds(chain)).toStrictEqual(["food", "population"]);
      expect(chain.levels[1]).toMatchObject({ dwell: 1, open: true, byKeyboard: true });
    });

    it("do nothing in a card still filling its bar", () => {
      chain.hoverOwner("food", element());
      vi.advanceTimersByTime(appear);
      chain.hoverLink(0, "population", element());
      chain.openLink(0, "population", element());
      vi.runAllTimers();
      expect(shownIds(chain)).toStrictEqual(["food"]);
    });

    it("show nothing when the pointer leaves them before the delay", () => {
      openByKeyboard(chain, "food");
      chain.hoverLink(0, "population", element());
      vi.advanceTimersByTime(appear - 1);
      chain.leaveLink(0);
      vi.runAllTimers();
      expect(shownIds(chain)).toStrictEqual(["food"]);
    });

    it("drop their card still filling its bar once the pointer leaves them, after the grace", () => {
      openByKeyboard(chain, "food");
      chain.hoverLink(0, "population", element());
      vi.advanceTimersByTime(appear);
      chain.leaveLink(0);
      vi.advanceTimersByTime(grace - 1);
      expect(shownIds(chain)).toStrictEqual(["food", "population"]);

      vi.advanceTimersByTime(1);
      expect(shownIds(chain)).toStrictEqual(["food"]);
      // nor does its dwell go on
      expect(vi.getTimerCount()).toBe(0);
    });

    it("keep their card already open once the pointer leaves them, for the pointer to enter it", () => {
      openByKeyboard(chain, "food");
      chain.hoverLink(0, "population", element());
      vi.advanceTimersByTime(appear + dwell + FRAME);
      chain.leaveLink(0);
      vi.runAllTimers();
      expect(shownIds(chain)).toStrictEqual(["food", "population"]);
    });

    it("keep their card when the pointer comes back to them from it", () => {
      openByKeyboard(chain, "food", "population");
      chain.leaveCard(1);
      chain.enterCard(0);
      chain.hoverLink(0, "population", element());
      vi.runAllTimers();
      expect(shownIds(chain)).toStrictEqual(["food", "population"]);
    });

    it("replace the deeper cards with their own", () => {
      openByKeyboard(chain, "food", "population", "housing");
      chain.hoverLink(0, "shortage", element());
      vi.advanceTimersByTime(appear);
      expect(shownIds(chain)).toStrictEqual(["food", "shortage"]);
    });

    it(`open no card past ${MAX_CONCEPT_DEPTH} cards deep`, () => {
      const ids = Object.keys(GAME_CONCEPTS).slice(0, MAX_CONCEPT_DEPTH + 1);
      openByKeyboard(chain, ...ids);
      expect(shownIds(chain)).toStrictEqual(ids.slice(0, MAX_CONCEPT_DEPTH));

      chain.hoverLink(MAX_CONCEPT_DEPTH - 1, ids[MAX_CONCEPT_DEPTH], element());
      vi.runAllTimers();
      expect(chain.levels).toHaveLength(MAX_CONCEPT_DEPTH);
    });
  });

  describe("cards", () => {
    it("close with their descendants after the grace once the pointer leaves them", () => {
      openByKeyboard(chain, "food", "population", "housing");
      chain.leaveCard(1);
      vi.advanceTimersByTime(grace - 1);
      expect(chain.levels).toHaveLength(3);

      vi.advanceTimersByTime(1);
      expect(shownIds(chain)).toStrictEqual(["food"]);
    });

    it.each([
      { depth: 1, where: "back into it" },
      { depth: 2, where: "into its child" },
    ])("stay when the pointer moves $where within the grace", ({ depth }) => {
      openByKeyboard(chain, "food", "population", "housing");
      chain.leaveCard(1);
      chain.enterCard(depth);
      vi.runAllTimers();
      expect(chain.levels).toHaveLength(3);
    });

    it("close all the same when the pointer moves into their parent", () => {
      openByKeyboard(chain, "food", "population", "housing");
      chain.leaveCard(1);
      chain.enterCard(0);
      vi.runAllTimers();
      expect(shownIds(chain)).toStrictEqual(["food"]);
    });
  });

  describe("closing", () => {
    it("goes back by closing the deepest card", () => {
      openByKeyboard(chain, "food", "population", "housing");
      chain.back();
      expect(shownIds(chain)).toStrictEqual(["food", "population"]);
    });

    it("goes back by also cancelling the card about to appear", () => {
      openByKeyboard(chain, "food", "population");
      chain.hoverLink(1, "housing", element());
      chain.back();
      vi.runAllTimers();
      expect(shownIds(chain)).toStrictEqual(["food"]);
    });

    // as on every Escape, with no card shown
    it("goes back without a card shown, by only cancelling the one about to appear", () => {
      chain.back();
      expect(chain.levels).toHaveLength(0);

      chain.hoverOwner("food", element());
      chain.back();
      vi.runAllTimers();
      expect(chain.levels).toHaveLength(0);
    });

    it("closes every card at once, and cancels the one about to appear", () => {
      openByKeyboard(chain, "food", "population");
      chain.hoverLink(1, "housing", element());
      chain.closeAll();
      expect(chain.levels).toHaveLength(0);

      vi.runAllTimers();
      expect(chain.levels).toHaveLength(0);
    });
  });
});

describe("conceptOwner", () => {
  it("shows the card of its element under the pointer, until the pointer leaves it", () => {
    const owner = element();
    conceptOwner(chain, "food")(owner);
    owner.dispatchEvent(new PointerEvent("pointerenter"));
    vi.advanceTimersByTime(appear);
    expect(shownIds(chain)).toStrictEqual(["food"]);
    expect(chain.levels[0].anchor).toBe(owner);

    owner.dispatchEvent(new PointerEvent("pointerleave"));
    vi.advanceTimersByTime(grace);
    expect(chain.levels).toHaveLength(0);
  });

  it("opens the card of its element under keyboard focus, until the focus leaves it", () => {
    const owner = element();
    conceptOwner(chain, "food")(owner);
    owner.dispatchEvent(new FocusEvent("focusin"));
    expect(shownIds(chain)).toStrictEqual(["food"]);
    expect(chain.levels[0]).toMatchObject({ anchor: owner, byKeyboard: true });

    owner.dispatchEvent(new FocusEvent("focusout"));
    vi.advanceTimersByTime(grace);
    expect(chain.levels).toHaveLength(0);
  });

  it("stops listening to its element once detached", () => {
    const owner = element();
    const detach = conceptOwner(chain, "food")(owner) as () => void;
    detach();
    owner.dispatchEvent(new PointerEvent("pointerenter"));
    owner.dispatchEvent(new FocusEvent("focusin"));
    vi.runAllTimers();
    expect(chain.levels).toHaveLength(0);
  });
});
