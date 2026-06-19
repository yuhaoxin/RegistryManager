import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { addAndSelectManualProfile } from "./profile-helpers";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("safe manifest delete uses simple confirm dialog", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await addAndSelectManualProfile(page, { catalog: true });
  await page.evaluate(() => {
    localStorage.removeItem("rm-mock-delete-404");
    localStorage.removeItem("rm-audit-events");
  });

  await page.getByRole("button", { name: /Open alpine/i }).click();
  await page.getByText("latest").click();
  await expect(page.getByTestId("rm-manifest-drawer")).toBeVisible();

  await page.getByTestId("delete-manifest-button").click();
  await expect(page.getByTestId("delete-confirm-dialog")).toBeVisible();
  await expect(page.getByTestId("delete-confirm-dialog")).toContainText("Delete this image tag from alpine?");
  await expect(page.getByTestId("delete-confirm-dialog")).toContainText("Storage is reclaimed only after registry GC");
  await expect(page.getByRole("button", { name: "Cancel" })).toBeVisible();
  await page.getByRole("button", { name: "Confirm" }).click();

  await expect(page.getByTestId("delete-confirm-dialog")).toContainText("pending_gc");
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("delete_manifest");
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("pending_gc");
  await page.screenshot({ path: path.join(evidenceDir, "task-8-safe-delete.png"), fullPage: true });
  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-safe-delete.png"), fullPage: true });
});
