import { describe, expect, it } from "vitest";

import { parseRoute } from "./hooks";

describe("parseRoute", () => {
  it("routes piece hashes to the piece view", () => {
    expect(parseRoute("#/pieces/boreal_pocket")).toEqual({ view: "piece", slug: "boreal_pocket" });
  });

  it("falls back to the piece list", () => {
    for (const hash of ["", "#/", "#/pieces/", "#/pieces/Bad Slug", "#/other"]) {
      expect(parseRoute(hash)).toEqual({ view: "pieces" });
    }
  });
});
