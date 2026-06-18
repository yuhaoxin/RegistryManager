import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("offline mode reopens profile with stale cached catalog", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await page.goto("/");
  await page.evaluate(() => {
    localStorage.removeItem("rm-mock-registry-offline");
    localStorage.removeItem("rm-selected-profile");
  });
  await page.reload();

  await page.getByTestId("rm-local-registry-container-picker").locator("label").first().click();
  await expect(page.getByTestId("rm-repository-card").first()).toBeVisible();
  await page.getByRole("button", { name: /Open alpine/i }).click();
  await expect(page.getByTestId("rm-tag-browser")).toContainText("latest");

  await page.evaluate(() => localStorage.setItem("rm-mock-registry-offline", "true"));
  await page.reload();

  await expect(page.getByTestId("stale-cache-banner").first()).toBeVisible();
  await expect(page.getByTestId("stale-cache-banner").first()).toContainText(/cache|offline|stale/i);
  await expect(page.getByTestId("rm-repository-card").first()).toContainText("alpine");

  await page.screenshot({ path: path.join(evidenceDir, "task-7-offline-cache.png"), fullPage: true });
  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-offline-cache.png"), fullPage: true });
});
