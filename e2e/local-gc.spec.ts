import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { addAndSelectManualProfile } from "./profile-helpers";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("local GC shows confirm dialog, timeline and logs", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await addAndSelectManualProfile(page, { gc: true });
  await page.evaluate(() => localStorage.removeItem("rm-audit-events"));

  await page.getByRole("button", { name: /^Run GC$/ }).click();
  await expect(page.getByTestId("rm-gc-confirm-dialog")).toContainText("Downtime warning");
  await page.getByTestId("rm-run-gc-button").click();

  await expect(page.getByTestId("rm-gc-step-timeline")).toContainText("health");
  await expect(page.getByTestId("rm-gc-step-timeline")).toContainText("restart");
  await expect(page.getByTestId("rm-gc-live-log-panel")).toContainText("garbage-collect --delete-untagged");
  await expect(page.getByTestId("rm-gc-live-log-panel")).toContainText("[stop]");
  await expect(page.getByTestId("rm-gc-live-log-panel")).toContainText("[cleanup]");
  await expect(page.getByTestId("rm-gc-result-summary")).toContainText("GC completed");
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("gc_completed");
  await page.screenshot({ path: path.join(evidenceDir, "task-8-local-gc.png"), fullPage: true });
  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-local-gc.png"), fullPage: true });
});
