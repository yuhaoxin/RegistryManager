import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("Docker daemon unavailable onboarding explains recovery and disables GC", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await page.goto("/");
  await page.evaluate(() => {
    localStorage.setItem("rm-mock-docker-unavailable", "true");
    localStorage.removeItem("rm-selected-profile");
  });
  await page.reload();

  await expect(page.getByTestId("rm-docker-status-card")).toContainText("Unavailable");
  await expect(page.getByTestId("rm-docker-unavailable-empty").first()).toContainText(/Start Docker Desktop|Docker Engine|docker run/i);
  await expect(page.getByTestId("rm-local-registry-container-picker")).toContainText("No local registry containers found");
  await expect(page.getByRole("button", { name: /^Run GC$/ })).toHaveCount(0);

  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-docker-unavailable.png"), fullPage: true });
});

test("healthy onboarding shows local registry picker", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => {
    localStorage.removeItem("rm-mock-docker-unavailable");
    localStorage.removeItem("rm-selected-profile");
  });
  await page.reload();

  await expect(page.getByTestId("rm-docker-status-card")).toContainText("Connected");
  await expect(page.getByTestId("rm-local-registry-container-picker")).toContainText("5000:5000");
});
