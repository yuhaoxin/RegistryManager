import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { addAndSelectManualProfile } from "./profile-helpers";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("local GC failure shows failed timeline, logs and recovery instructions", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await addAndSelectManualProfile(page, { gc: true });
  await page.evaluate(() => {
    localStorage.removeItem("rm-audit-events");
    localStorage.setItem("rm-mock-gc-failure", "true");
  });

  await page.getByRole("button", { name: /^运行 GC$/ }).click();
  await expect(page.getByTestId("rm-gc-confirm-dialog")).toContainText("停机警告");
  await page.getByTestId("rm-run-gc-button").click();

  await expect(page.getByTestId("rm-gc-step-timeline")).toContainText("执行 GC");
  await expect(page.getByTestId("rm-gc-step-timeline")).toContainText("无效");
  await expect(page.getByTestId("rm-gc-live-log-panel")).toContainText("/missing/config.yml");
  await expect(page.getByTestId("rm-gc-result-summary")).toContainText("GC 失败");
  await expect(page.getByText(/需要恢复/)).toContainText("docker start registry");
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("GC 失败");

  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-local-gc-failure.png"), fullPage: true });
  await page.evaluate(() => localStorage.removeItem("rm-mock-gc-failure"));
});
