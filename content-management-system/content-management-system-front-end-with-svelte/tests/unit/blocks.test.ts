// The block model: the editor's whole vocabulary, tested without
// mounting anything.

import { describe, expect, it } from "vitest";
import {
  BLOCK_KINDS,
  blank,
  documentProblems,
  insertAt,
  move,
  preview,
  problems,
  removeAt,
  replaceAt,
} from "$lib/blocks";

describe("the block model", () => {
  it("creates every kind it offers, with that kind's fields", () => {
    for (const kind of BLOCK_KINDS) {
      const block = blank(kind);
      expect(block.kind).toBe(kind);
      // A newly created block is never *invalid* in shape, only empty.
      expect(
        problems(block).every((p) => !p.includes("not a block kind")),
      ).toBe(true);
    }
    expect(blank("heading").level).toBe(2);
    // Alt text starts present-but-empty so the editor sees the field.
    expect(blank("image").alt).toBe("");
  });

  it("inserts, removes, and replaces without mutating the input", () => {
    const original = [blank("paragraph"), blank("quote")];
    const frozen = JSON.stringify(original);

    expect(insertAt(original, 1, blank("code"))).toHaveLength(3);
    expect(insertAt(original, 99, blank("code"))[2]?.kind).toBe("code");
    expect(insertAt(original, -5, blank("code"))[0]?.kind).toBe("code");
    expect(removeAt(original, 0)).toHaveLength(1);
    expect(removeAt(original, 99)).toHaveLength(2);
    expect(replaceAt(original, 0, blank("list"))[0]?.kind).toBe("list");
    expect(replaceAt(original, 99, blank("list"))[0]?.kind).toBe("paragraph");

    expect(JSON.stringify(original)).toBe(frozen);
  });

  it("moves a block, and leaves the list alone when the drag lands outside", () => {
    const blocks = [blank("heading"), blank("paragraph"), blank("quote")];
    expect(move(blocks, 0, 2).map((b) => b.kind)).toEqual([
      "paragraph",
      "quote",
      "heading",
    ]);
    expect(move(blocks, 2, 0).map((b) => b.kind)).toEqual([
      "quote",
      "heading",
      "paragraph",
    ]);
    // A drag that ends nowhere must do nothing — clamping would drop
    // the block somewhere the editor never pointed at.
    for (const [from, to] of [
      [0, 9],
      [9, 0],
      [-1, 1],
      [1, -1],
      [1, 1],
    ]) {
      expect(move(blocks, from!, to!)).toEqual(blocks);
    }
  });

  it("summarises a block as text, never as markup", () => {
    expect(preview({ kind: "paragraph", text: "hello" })).toBe("hello");
    expect(preview({ kind: "list", items: ["a", "b"] })).toBe("a, b");
    expect(preview({ kind: "image", alt: "a cat" })).toBe("a cat");
    expect(preview({ kind: "paragraph", text: "x".repeat(200) }).length).toBe(
      80,
    );
    // A block whose text contains markup is summarised verbatim; it is
    // never interpreted, and the UI never renders it as HTML.
    expect(preview({ kind: "paragraph", text: "<b>x</b>" })).toBe("<b>x</b>");
  });

  it("explains what an image is missing in terms of what it blocks", () => {
    const image = { kind: "image", asset_pid: "", alt: "" };
    const found = problems(image);
    expect(found.some((p) => p.includes("choose an image"))).toBe(true);
    expect(found.some((p) => p.includes("published"))).toBe(true);

    const described = { kind: "image", asset_pid: "a", alt: "a chart" };
    expect(problems(described)).toEqual([]);
    // Whitespace is not alt text.
    expect(
      problems({ kind: "image", asset_pid: "a", alt: "   " }),
    ).toHaveLength(1);
  });

  it("refuses a kind the editor cannot create, and says so once", () => {
    const found = problems({ kind: "iframe", src: "https://x.test" });
    expect(found).toHaveLength(1);
    expect(found[0]).toContain("not a block kind");
  });

  it("checks heading levels against what a document may contain", () => {
    expect(problems({ kind: "heading", level: 2, text: "x" })).toEqual([]);
    expect(problems({ kind: "heading", level: 6, text: "x" })).toEqual([]);
    // Level 1 is the page title; the body does not repeat it.
    expect(problems({ kind: "heading", level: 1, text: "x" })).toHaveLength(1);
    expect(problems({ kind: "heading", level: 7, text: "x" })).toHaveLength(1);
    expect(problems({ kind: "heading", text: "x" })).toHaveLength(1);
  });

  it("reports document problems with the position that caused them", () => {
    const blocks = [
      { kind: "paragraph", text: "fine" },
      { kind: "image", asset_pid: "", alt: "" },
    ];
    const found = documentProblems(blocks);
    expect(found.every((f) => f.index === 1)).toBe(true);
    expect(found).toHaveLength(2);
    expect(documentProblems([{ kind: "paragraph", text: "ok" }])).toEqual([]);
  });
});
