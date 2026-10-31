import { describe, expect, it } from "vitest";

import { conceptText } from "./concepts";

// `toStrictEqual` throughout: a link without shown words must have no `text` key, not an undefined one
describe("conceptText", () => {
  it("parses an empty summary into no segments", () => {
    expect(conceptText("")).toStrictEqual([]);
  });

  it("keeps prose without links as a single text segment", () => {
    expect(conceptText("What a building costs every turn.")).toStrictEqual([
      { text: "What a building costs every turn." },
    ]);
  });

  it("parses a link alone", () => {
    expect(conceptText("[[food]]")).toStrictEqual([{ concept: "food" }]);
  });

  it("splits the prose around a link", () => {
    expect(conceptText("Burns [[uranium]] into electricity")).toStrictEqual([
      { text: "Burns " },
      { concept: "uranium" },
      { text: " into electricity" },
    ]);
  });

  it("parses a link with shown words", () => {
    expect(conceptText("Spent by [[shortage|shortages]] and war.")).toStrictEqual([
      { text: "Spent by " },
      { concept: "shortage", text: "shortages" },
      { text: " and war." },
    ]);
  });

  it("keeps spaces in the shown words", () => {
    expect(conceptText("[[settlement|the settlements]]")).toStrictEqual([
      { concept: "settlement", text: "the settlements" },
    ]);
  });

  it("parses hyphenated concept ids", () => {
    expect(conceptText("Fuels a [[fission-reactor]].")).toStrictEqual([
      { text: "Fuels a " },
      { concept: "fission-reactor" },
      { text: "." },
    ]);
  });

  it("adds no empty text segments around links at the edges", () => {
    expect(conceptText("[[food]] and [[water]]")).toStrictEqual([
      { concept: "food" },
      { text: " and " },
      { concept: "water" },
    ]);
  });

  it("adds no empty text segment between adjacent links", () => {
    expect(conceptText("[[metals]][[uranium|ore]]")).toStrictEqual([
      { concept: "metals" },
      { concept: "uranium", text: "ore" },
    ]);
  });

  it("keeps the words glued to a link in the prose", () => {
    expect(conceptText("Where a [[nation]]'s [[population]] lives")).toStrictEqual([
      { text: "Where a " },
      { concept: "nation" },
      { text: "'s " },
      { concept: "population" },
      { text: " lives" },
    ]);
  });

  it.each(["[food]", "[[food", "food]]", "[[]]", "[[|words]]", "[[food|]]"])(
    "leaves the malformed link %s as plain text",
    (markup) => {
      expect(conceptText(`Feeds ${markup} people`)).toStrictEqual([{ text: `Feeds ${markup} people` }]);
    },
  );

  it("does not let an unclosed link swallow the next one", () => {
    expect(conceptText("[[food [[water]]")).toStrictEqual([{ text: "[[food " }, { concept: "water" }]);
    expect(conceptText("[[food|some [[water]]")).toStrictEqual([{ text: "[[food|some " }, { concept: "water" }]);
  });
});
