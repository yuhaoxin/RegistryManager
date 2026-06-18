import { expect, test } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import path from "node:path";

const evidenceDir = path.join(process.cwd(), ".sisyphus", "evidence");

test("safe delete displays specific mocked 404 error", async ({ page }) => {
  await mkdir(evidenceDir, { recursive: true });
  await page.goto("/");
  await page.evaluate(() => {
    localStorage.setItem("rm-mock-delete-404", "true");
    localStorage.removeItem("rm-audit-events");
  });

  await page.getByTestId("rm-local-registry-container-picker").locator("label").first().click();
  await page.getByRole("button", { name: /Open alpine/i }).click();
  await page.getByText("latest").click();
  await page.getByTestId("delete-manifest-button").click();
  await expect(page.getByTestId("delete-confirm-dialog")).toContainText("Delete this image tag from alpine?");
  await page.getByRole("button", { name: "Confirm" }).click();

  await expect(page.getByTestId("delete-confirm-dialog")).toContainText("Manifest digest was not found");
  await expect(page.getByTestId("rm-audit-log-table")).toContainText("failure");
  await page.screenshot({ path: path.join(evidenceDir, "task-8-safe-delete-error.png"), fullPage: true });
});
