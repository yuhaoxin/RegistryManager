import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { addAndSelectManualProfile } from "./profile-helpers";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("local GC shows confirm dialog, timeline and logs", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await addAndSelectManualProfile(page, { gc: true });
  await page.evaluate(() => localStorage.removeItem("rm-audit-events"));

  await page.getByRole("button", { name: /^运行 GC$/ }).click();
  await expect(page.getByTestId("rm-gc-confirm-dialog")).toContainText("停机警告");
  await page.getByTestId("rm-run-gc-button").click();

  await expect(page.getByTestId("rm-gc-step-timeline")).toContainText("健康检查");
  await expect(page.getByTestId("rm-gc-step-timeline")).toContainText("重启容器");
  await expect(page.getByTestId("rm-gc-live-log-panel")).toContainText("garbage-collect --delete-untagged");
  await expect(page.getByTestId("rm-gc-live-log-panel")).toContainText("[stop]");
  await expect(page.getByTestId("rm-gc-live-log-panel")).toContainText("[cleanup]");
  await expect(page.getByTestId("rm-gc-result-summary")).toContainText("GC 已完成");
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("GC 已完成");
  await page.screenshot({ path: path.join(evidenceDir, "task-8-local-gc.png"), fullPage: true });
  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-local-gc.png"), fullPage: true });
});
