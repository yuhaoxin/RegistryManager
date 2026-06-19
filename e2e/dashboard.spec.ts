import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { addAndSelectManualProfile, resetMockState } from "./profile-helpers";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

async function ensureEvidenceDir() {
  await mkdir(evidenceDir, { recursive: true });
}

test("dashboard empty state screenshot", async ({ page }) => {
  await ensureEvidenceDir();
  await resetMockState(page);

  await expect(page.getByTestId("app-root")).toBeVisible();
  await expect(page.getByTestId("rm-docker-status-card")).toBeVisible();
  await expect(page.getByTestId("rm-docker-unavailable-empty")).toBeVisible();

  await page.screenshot({
    path: path.join(evidenceDir, "task-6-dashboard-empty.png"),
    fullPage: true,
  });
});

test("dashboard search no results screenshot", async ({ page }) => {
  await ensureEvidenceDir();
  await addAndSelectManualProfile(page, { catalog: true });

  await expect(page.getByTestId("app-root")).toBeVisible();
  await expect(page.getByTestId("rm-repository-search")).toBeVisible();

  const searchInput = page.getByTestId("rm-repository-search").locator("input");
  await searchInput.fill("no-such-repo");
  await expect(page.getByTestId("no-search-results")).toBeVisible();

  await page.screenshot({
    path: path.join(evidenceDir, "task-6-dashboard-search-error.png"),
    fullPage: true,
  });
});
