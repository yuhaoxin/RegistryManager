import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { addAndSelectManualProfile, resetMockState } from "./profile-helpers";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("Docker daemon unavailable onboarding explains recovery and disables GC", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await resetMockState(page);
  await page.evaluate(() => {
    localStorage.setItem("rm-mock-docker-unavailable", "true");
    localStorage.removeItem("rm-selected-profile");
  });
  await page.reload();

  await expect(page.getByTestId("rm-docker-status-card")).toContainText("不可用");
  await expect(page.getByTestId("rm-docker-status-empty")).toContainText(/Docker 守护进程|Docker Engine|Docker Desktop/);
  await expect(page.getByTestId("rm-no-profiles-message")).toContainText("还没有 Registry 配置");
  await expect(page.getByRole("button", { name: /^运行 GC$/ })).toHaveCount(0);

  await page.screenshot({ path: path.join(evidenceDir, "task-9-e2e-docker-unavailable.png"), fullPage: true });
});

test("healthy onboarding adds and selects manual registry profile", async ({ page }) => {
  await addAndSelectManualProfile(page, { catalog: true });

  await expect(page.getByTestId("rm-profile-list")).toContainText("http://localhost:5000");
  await expect(page.getByTestId("rm-repository-browser")).toBeVisible();
});
