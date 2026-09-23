import { describe, expect, it } from "vitest";

import { formatBytes, formatDate, sourceLabel } from "./format";

describe("format", () => {
  it("formats byte sizes", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(3 * 1024 * 1024)).toBe("3.0 MB");
  });

  it("omits the year for dates in the current year", () => {
    const now = new Date("2026-09-23T12:00:00Z");
    expect(formatDate("2026-09-22T08:00:00Z", now)).not.toContain("2026");
    expect(formatDate("2025-01-02T08:00:00Z", now)).toContain("2025");
  });

  it("labels version sources", () => {
    expect(sourceLabel("owner")).toBe("your edit");
    expect(sourceLabel("agent")).toBe("published");
  });
});
