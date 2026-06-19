import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("RegistryProfile contract", () => {
  it("exposes manual URL profile fields", () => {
    const profileSource = registryProfileInterfaceSource();

    expectField(profileSource, "id");
    expectField(profileSource, "name");
    expectField(profileSource, "registryUrl");
    expectField(profileSource, "credentialRef", "(?:string(?:\\s*\\|\\s*null)?|null\\s*\\|\\s*string)");
    expectField(profileSource, "containerId", "(?:string(?:\\s*\\|\\s*null)?|null\\s*\\|\\s*string)");
    expectField(profileSource, "containerName", "(?:string(?:\\s*\\|\\s*null)?|null\\s*\\|\\s*string)");
    expectField(profileSource, "createdAt");
    expectField(profileSource, "updatedAt");
  });

  it("does not expose persisted status fields", () => {
    const profileSource = registryProfileInterfaceSource();

    for (const field of [
      "image",
      "portMapping",
      "storageMounts",
      "selectedAt",
      "lastHealthCheckAt",
      "healthStatus",
      "status",
    ]) {
      expect(profileSource).not.toMatch(new RegExp(`\\b${field}\\??:`));
    }
  });
});

function registryProfileInterfaceSource(): string {
  const typesPath = resolve(process.cwd(), "src/types.ts");
  const typesSource = readFileSync(typesPath, "utf8");
  const match = typesSource.match(/export interface RegistryProfile\s*{(?<body>[\s\S]*?)\n}/);

  expect(match).not.toBeNull();
  return match?.groups?.body ?? "";
}

function expectField(source: string, field: string, typePattern = "string") {
  expect(source).toMatch(new RegExp(`\\b${field}\\??:\\s*${typePattern}\\b`));
}
