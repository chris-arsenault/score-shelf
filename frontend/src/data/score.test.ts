import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../auth", () => ({ getAccessToken: vi.fn(async () => "owner-token") }));

import { ApiError, resetFetchForTests, setFetchForTests } from "./api";
import { loadScoreFile } from "./shelf";

afterEach(resetFetchForTests);

describe("private score loading", () => {
  it("authorizes the URL request but sends no owner credentials to S3, preserving binary MXL", async () => {
    const bytes = new Uint8Array([80, 75, 3, 4, 0, 255, 128]);
    const calls: RequestInit[] = [];
    setFetchForTests(async (input, init) => {
      calls.push(init ?? {});
      if (String(input).endsWith("/files/score-id/download")) {
        return Response.json({ url: "https://s3.test/score.mxl?signed=yes" });
      }
      expect(String(input)).toBe("https://s3.test/score.mxl?signed=yes");
      return new Response(bytes);
    });
    const signal = new AbortController().signal;
    const file = await loadScoreFile("score-id", signal);
    expect(calls[0].headers).toEqual({ Authorization: "Bearer owner-token" });
    expect(calls[1]).toEqual({ credentials: "omit", signal });
    expect(new Uint8Array(await file.arrayBuffer())).toEqual(bytes);
  });

  it("reports a rejected S3 download", async () => {
    setFetchForTests(async (input) =>
      String(input).includes("/files/")
        ? Response.json({ url: "https://s3.test/score" })
        : new Response("denied", { status: 403 })
    );
    await expect(loadScoreFile("score-id", new AbortController().signal)).rejects.toEqual(
      new ApiError(403, "download_failed", "The score could not be downloaded.")
    );
  });

  it("does not start an S3 transfer after the viewer has closed", async () => {
    const controller = new AbortController();
    const transport = vi.fn(async () => {
      controller.abort();
      return Response.json({ url: "https://s3.test/score" });
    });
    setFetchForTests(transport);
    await expect(loadScoreFile("score-id", controller.signal)).rejects.toThrow();
    expect(transport).toHaveBeenCalledTimes(1);
  });
});
