import { describe, expect, it } from "vitest";
import { isLocalRegistryUrl } from "./registryUrl";

describe("isLocalRegistryUrl", () => {
  it.each([
    "http://localhost:5000",
    "https://localhost:5000/v2/",
    "http://127.0.0.1:5000",
    "http://127.1.2.3:5000",
    "http://[::1]:5000",
  ])("returns true for %s", (url) => {
    expect(isLocalRegistryUrl(url)).toBe(true);
  });

  it.each([
    "https://registry.example.com",
    "http://192.168.1.1:5000",
    "http://10.0.0.1:5000",
  ])("returns false for %s", (url) => {
    expect(isLocalRegistryUrl(url)).toBe(false);
  });
});
