import { expect, test } from "@playwright/test";

test("renders the web app shell", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByTestId("app-root")).toBeVisible();
  await expect(page.getByText("Registry Manager")).toBeVisible();
  await expect(page.getByTestId("rm-docker-status-card")).toBeVisible();
});
