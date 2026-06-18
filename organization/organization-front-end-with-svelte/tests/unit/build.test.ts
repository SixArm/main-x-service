import { describe, it, expect } from "vitest";
import {
  buildOrganization,
  splitList,
  blankToUndef,
  excludeSelf,
} from "$lib/api/build";
import type { OrganizationFormInputs } from "$lib/api/build";
import type { ScoredRef } from "$lib/api/types";

// Pins the spec §8 form->payload core (comma-list splitting, blank->null
// clearing, all-or-nothing address assembly, dropping empty identifier
// rows) and the §6.6 self-match exclusion. These are the most
// logic-heavy units in the project; they live in $lib/api/build.ts so
// they are testable without mounting the Svelte component.

/** A fully-blank set of form inputs; spread + override per case. */
function blankInputs(): OrganizationFormInputs {
  return {
    name: "",
    legalName: "",
    url: "",
    jurisdiction: "",
    foundingDate: "",
    telephone: "",
    email: "",
    alternateNames: "",
    keywords: "",
    sameAs: "",
    identifiers: [],
    street: "",
    locality: "",
    region: "",
    postalCode: "",
    country: "",
  };
}

describe("splitList", () => {
  it("splits, trims, and drops empties", () => {
    expect(splitList("a, b ,  ,c")).toEqual(["a", "b", "c"]);
  });

  it("returns an empty array for a blank or empty input", () => {
    expect(splitList("")).toEqual([]);
    expect(splitList("  ,  , ")).toEqual([]);
  });
});

describe("blankToUndef", () => {
  it("trims a non-empty value", () => {
    expect(blankToUndef("  hi  ")).toBe("hi");
  });

  it("collapses a blank value to undefined", () => {
    expect(blankToUndef("   ")).toBeUndefined();
    expect(blankToUndef("")).toBeUndefined();
  });
});

describe("buildOrganization", () => {
  it("trims the required name and clears blank scalars to null", () => {
    const org = buildOrganization({ ...blankInputs(), name: "  Acme  " });
    expect(org.name).toBe("Acme");
    // Blank scalars become explicit null (cleared), not undefined.
    expect(org.legal_name).toBeNull();
    expect(org.url).toBeNull();
    expect(org.jurisdiction).toBeNull();
    expect(org.founding_date).toBeNull();
    expect(org.telephone).toBeNull();
    expect(org.email).toBeNull();
  });

  it("carries contact fields (telephone + email) when present", () => {
    const org = buildOrganization({
      ...blankInputs(),
      name: "Acme",
      telephone: "  +1 555 0100 ",
      email: " ops@acme.test ",
    });
    expect(org.telephone).toBe("+1 555 0100");
    expect(org.email).toBe("ops@acme.test");
  });

  it("splits the comma-separated list fields", () => {
    const org = buildOrganization({
      ...blankInputs(),
      name: "Acme",
      alternateNames: "ACME, Acme Inc",
      keywords: "widgets, gadgets",
      sameAs: "https://a.test, https://b.test",
    });
    expect(org.alternate_names).toEqual(["ACME", "Acme Inc"]);
    expect(org.keywords).toEqual(["widgets", "gadgets"]);
    expect(org.same_as).toEqual(["https://a.test", "https://b.test"]);
  });

  it("omits the address entirely when no part is filled", () => {
    const org = buildOrganization({ ...blankInputs(), name: "Acme" });
    expect(org.address).toBeUndefined();
  });

  it("emits the address when at least one part is filled, nulling the rest", () => {
    const org = buildOrganization({
      ...blankInputs(),
      name: "Acme",
      locality: "  London  ",
    });
    expect(org.address).toEqual({
      street_address: null,
      locality: "London",
      region: null,
      postal_code: null,
      country: null,
    });
  });

  it("assembles a full address", () => {
    const org = buildOrganization({
      ...blankInputs(),
      name: "Acme",
      street: "1 High St",
      locality: "London",
      region: "England",
      postalCode: "SW1A 1AA",
      country: "GB",
    });
    expect(org.address).toEqual({
      street_address: "1 High St",
      locality: "London",
      region: "England",
      postal_code: "SW1A 1AA",
      country: "GB",
    });
  });

  it("drops identifier rows with empty values and trims kept ones", () => {
    const org = buildOrganization({
      ...blankInputs(),
      name: "Acme",
      identifiers: [
        { scheme: "Lei", value: "  529900T8BM49AURSDO55  " },
        { scheme: "Duns", value: "   " },
        { scheme: "Vat", value: "" },
      ],
    });
    expect(org.identifiers).toEqual([
      { scheme: "Lei", value: "529900T8BM49AURSDO55" },
    ]);
  });

  it("emits an empty identifiers array when every row is blank", () => {
    const org = buildOrganization({
      ...blankInputs(),
      name: "Acme",
      identifiers: [{ scheme: "Lei", value: "  " }],
    });
    expect(org.identifiers).toEqual([]);
  });
});

describe("excludeSelf", () => {
  const hits: ScoredRef[] = [
    { pid: "self", name: "Acme", score: 1, confidence: "Certain", is_match: true },
    { pid: "other", name: "Acme Co", score: 0.9, confidence: "Probable", is_match: true },
  ];

  it("drops the record's own pid (spec §6.6 self-match exclusion)", () => {
    const out = excludeSelf(hits, "self");
    expect(out.map((h) => h.pid)).toEqual(["other"]);
  });

  it("is a no-op when the self pid is not among the hits", () => {
    const out = excludeSelf(hits, "absent");
    expect(out.map((h) => h.pid)).toEqual(["self", "other"]);
  });
});
