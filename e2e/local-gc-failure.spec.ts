import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("local GC failure shows failed timeline, logs and recovery instructions", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await page.goto("/");
  await page.evaluate(() => {
    localStorage.removeItem("rm-audit-events");
    localStorage.setItem("rm-mock-gc-failure", "true");
  });

  await page.getByTestId("rm-local-registry-container-picker").locator("label").first().click();
  await page.getByRole("button", { name: /^Run GC$/ }).click();
  await expect(page.getByTestId("rm-gc-confirm-dialog")).toContainText("Downtime warning");
  await page.getByTestId("rm-run-gc-button").click();

  await expect(page.getByTestId("rm-gc-step-timeline")).toContainText("gc");
  await expect(page.getByTestId("rm-gc-step-timeline")).toContainText("invalid");
  await expect(page.getByTestId("rm-gc-live-log-panel")).toContainText("/missing/config.yml");
  await expect(page.getByTestId("rm-gc-result-summary")).toContainText("GC failed");
  await expect(page.getByText(/Recovery required/i)).toContainText("docker start registry");
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("gc_failed");

  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-local-gc-failure.png"), fullPage: true });
  await page.evaluate(() => localStorage.removeItem("rm-mock-gc-failure"));
});
