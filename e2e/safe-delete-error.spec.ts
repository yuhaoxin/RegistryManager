import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { addAndSelectManualProfile } from "./profile-helpers";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("safe delete displays specific mocked 404 error", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await addAndSelectManualProfile(page, { catalog: true });
  await page.evaluate(() => {
    localStorage.setItem("rm-mock-delete-404", "true");
    localStorage.removeItem("rm-audit-events");
  });

  await page.getByRole("button", { name: /打开 alpine/ }).click();
  await page.getByText("latest").click();
  await page.getByTestId("delete-manifest-button").click();
  await expect(page.getByTestId("delete-confirm-dialog")).toContainText("从 alpine 删除此镜像标签？");
  await page.getByRole("button", { name: "确认" }).click();

  await expect(page.getByTestId("delete-confirm-dialog")).toContainText("Registry 中未找到清单摘要");
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("失败");
  await page.screenshot({ path: path.join(evidenceDir, "task-8-safe-delete-error.png"), fullPage: true });
});
