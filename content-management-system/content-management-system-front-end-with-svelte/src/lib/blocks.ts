// The structured block model, as the editor manipulates it.
//
// The service stores **blocks, never HTML** (`../spec/authoring.md`).
// This module is the whole client-side block vocabulary: pure
// functions over an array, so the editor's behaviour is testable
// without mounting anything, and so no code path can quietly turn a
// document into markup.
//
// There is deliberately no `toHtml` here, and no `fromHtml`. A helper
// that serialized blocks to a string would be used, and the moment it
// is used the round trip stops being lossless — which is the property
// the block model exists to guarantee.

import type { Block } from "./api/cms";

/** Block kinds the editor can create. The service accepts these; a
 *  kind it does not know is refused at write time, so offering more
 *  here would only produce a later `422`. */
export const BLOCK_KINDS = [
  "heading",
  "paragraph",
  "list",
  "quote",
  "image",
  "code",
] as const;

/** A kind the editor can create. */
export type BlockKind = (typeof BLOCK_KINDS)[number];

/** A new block of `kind`, with the fields that kind requires. */
export function blank(kind: BlockKind): Block {
  switch (kind) {
    case "heading":
      return { kind, level: 2, text: "" };
    case "list":
      return { kind, style: "unordered", items: [""] };
    case "image":
      // `alt` starts present-but-empty rather than absent: an editor
      // who sees the field is far likelier to fill it in than one who
      // meets the publish gate later and has to work out what it
      // wanted (`../spec/assets.md`).
      return { kind, asset_pid: "", alt: "" };
    case "code":
      return { kind, language: "text", text: "" };
    case "quote":
      return { kind, text: "", attribution: "" };
    default:
      return { kind, text: "" };
  }
}

/** Insert `block` at `index` (clamped), returning a new array. */
export function insertAt(
  blocks: Block[],
  index: number,
  block: Block,
): Block[] {
  const at = Math.max(0, Math.min(index, blocks.length));
  return [...blocks.slice(0, at), block, ...blocks.slice(at)];
}

/** Remove the block at `index`; out of range leaves the list alone. */
export function removeAt(blocks: Block[], index: number): Block[] {
  if (index < 0 || index >= blocks.length) return blocks;
  return [...blocks.slice(0, index), ...blocks.slice(index + 1)];
}

/**
 * Move the block at `from` to `to`.
 *
 * Out-of-range indices return the list unchanged rather than throwing
 * or silently clamping: a drag that ends outside the list should do
 * nothing, and clamping would move a block somewhere the editor did
 * not point at.
 */
export function move(blocks: Block[], from: number, to: number): Block[] {
  if (from < 0 || from >= blocks.length) return blocks;
  if (to < 0 || to >= blocks.length) return blocks;
  if (from === to) return blocks;
  const next = [...blocks];
  const [moved] = next.splice(from, 1);
  if (moved === undefined) return blocks;
  next.splice(to, 0, moved);
  return next;
}

/** Replace the block at `index`, returning a new array. */
export function replaceAt(
  blocks: Block[],
  index: number,
  block: Block,
): Block[] {
  if (index < 0 || index >= blocks.length) return blocks;
  return blocks.map((existing, at) => (at === index ? block : existing));
}

/** The text a block shows, for summaries and search. Never markup. */
export function preview(block: Block, limit = 80): string {
  const text =
    typeof block.text === "string"
      ? block.text
      : Array.isArray(block.items)
        ? block.items.filter((i) => typeof i === "string").join(", ")
        : typeof block.alt === "string"
          ? block.alt
          : "";
  return text.length > limit ? `${text.slice(0, limit - 1)}…` : text;
}

/** What this block would be refused for, in the editor, before the
 *  service says so. Advisory: the service is the authority, and these
 *  messages exist to save a round trip, not to replace its answer. */
export function problems(block: Block): string[] {
  const found: string[] = [];
  const kind = String(block.kind ?? "");
  if (!(BLOCK_KINDS as readonly string[]).includes(kind)) {
    found.push(`"${kind}" is not a block kind this editor can create`);
    return found;
  }
  if (kind === "heading") {
    const level = Number(block.level);
    if (!Number.isInteger(level) || level < 2 || level > 6) {
      // Level 1 is the page title, which the document does not repeat.
      found.push("a heading level must be between 2 and 6");
    }
  }
  if (kind === "image") {
    if (!block.asset_pid) found.push("choose an image");
    if (typeof block.alt !== "string" || block.alt.trim().length === 0) {
      // Stated as the consequence, not as a scolding: this is the one
      // gate that stops the page publishing.
      found.push("alt text is required before this page can be published");
    }
  }
  if (kind === "list") {
    const items = Array.isArray(block.items) ? block.items : [];
    if (items.filter((i) => String(i).trim().length > 0).length === 0) {
      found.push("a list needs at least one item");
    }
  }
  if (
    (kind === "paragraph" || kind === "quote" || kind === "code") &&
    String(block.text ?? "").trim().length === 0
  ) {
    found.push("this block is empty");
  }
  return found;
}

/** Every block's problems, flattened with its position. */
export function documentProblems(
  blocks: Block[],
): { index: number; problem: string }[] {
  return blocks.flatMap((block, index) =>
    problems(block).map((problem) => ({ index, problem })),
  );
}
