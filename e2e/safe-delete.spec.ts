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

  await page.getByRole("button", { name: /打开 alpine/ }).click();
  await page.getByText("latest").click();
  await expect(page.getByTestId("rm-manifest-drawer")).toBeVisible();

  await page.getByTestId("delete-manifest-button").click();
  await expect(page.getByTestId("delete-confirm-dialog")).toBeVisible();
  await expect(page.getByTestId("delete-confirm-dialog")).toContainText("从 alpine 删除此镜像标签？");
  await expect(page.getByTestId("delete-confirm-dialog")).toContainText("只有在 Registry GC 后才会回收存储空间");
  await expect(page.getByRole("button", { name: "取消" })).toBeVisible();
  await page.getByRole("button", { name: "确认" }).click();

  await expect(page.getByTestId("delete-confirm-dialog")).toBeHidden();
  await expect(page.getByTestId("rm-manifest-drawer")).toBeHidden();
  await expect(page.getByRole("cell", { name: "latest", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /打开 alpine/ })).toHaveCount(0);
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("删除清单");
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("等待 GC");
  await page.screenshot({ path: path.join(evidenceDir, "task-8-safe-delete.png"), fullPage: true });
  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-safe-delete.png"), fullPage: true });
});
