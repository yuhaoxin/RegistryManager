import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("select existing localhost:5000 registry and browse manifest", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await page.goto("/");
  await page.evaluate(() => localStorage.removeItem("rm-mock-registry-offline"));

  await expect(page.getByTestId("rm-local-registry-container-picker")).toBeVisible();
  await page.getByTestId("rm-local-registry-container-picker").locator("label").first().click();

  await expect(page.getByTestId("rm-registry-container-card")).toBeVisible();
  await expect(page.getByTestId("rm-registry-container-card")).toContainText("5000:5000");
  await expect(page.getByTestId("rm-repository-browser")).toBeVisible();
  await expect(page.getByTestId("rm-repository-card").first()).toBeVisible();

  await page.getByRole("button", { name: /Open alpine/i }).click();
  await expect(page.getByTestId("rm-tag-browser")).toContainText("latest");
  await page.getByText("latest").click();
  await expect(page.getByTestId("rm-manifest-drawer")).toBeVisible();
  await expect(page.getByTestId("rm-manifest-drawer")).toContainText("sha256:abc123def4567890");
  await expect(page.getByTestId("rm-manifest-drawer")).toContainText("application/vnd.docker.distribution.manifest.v2+json");
  await expect(page.getByTestId("rm-manifest-drawer")).toContainText("sha256:layer1");
  await expect(page.getByTestId("rm-manifest-drawer")).toContainText("linux/arm64");

  await page.screenshot({ path: path.join(evidenceDir, "task-7-browse.png"), fullPage: true });
  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-browse.png"), fullPage: true });
});
