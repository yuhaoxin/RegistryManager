import { expect, test } from "@playwright/test";

test("discovers and selects the existing localhost:5000 registry container", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => {
    localStorage.removeItem("rm-mock-docker-unavailable");
    localStorage.removeItem("rm-selected-profile");
  });
  await page.reload();

  const picker = page.getByTestId("rm-local-registry-container-picker");
  await expect(picker).toContainText("registry");
  await expect(picker).toContainText("5000:5000");

  await picker.locator("label").first().click();
  await expect(page.getByTestId("rm-registry-container-card")).toContainText("registry");
  await expect(page.getByTestId("rm-registry-container-card")).toContainText("5000:5000");
});
