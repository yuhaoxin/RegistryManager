import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

async function ensureEvidenceDir() {
  await mkdir(evidenceDir, { recursive: true });
}

test("dashboard empty state screenshot", async ({ page }) => {
  await ensureEvidenceDir();
  await page.goto("/");

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
  await page.goto("/");

  await expect(page.getByTestId("app-root")).toBeVisible();

  // Select the local registry container so the repository browser appears
  await page.getByTestId("rm-local-registry-container-picker").locator("label").click();
  await expect(page.getByTestId("rm-repository-search")).toBeVisible();

  const searchInput = page.getByTestId("rm-repository-search").locator("input");
  await searchInput.fill("no-such-repo");
  await expect(page.getByTestId("no-search-results")).toBeVisible();

  await page.screenshot({
    path: path.join(evidenceDir, "task-6-dashboard-search-error.png"),
    fullPage: true,
  });
});
