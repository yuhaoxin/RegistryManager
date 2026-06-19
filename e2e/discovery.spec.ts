import { expect, test } from "@playwright/test";
import { addAndSelectManualProfile } from "./profile-helpers";

test("adds and selects a manual localhost registry profile", async ({ page }) => {
  const profile = await addAndSelectManualProfile(page, { catalog: true });

  await expect(page.getByTestId("rm-profile-list")).toContainText(profile.name);
  await expect(page.getByTestId("rm-profile-list")).toContainText(profile.url);
  await expect(page.getByTestId("rm-repository-browser")).toBeVisible();
});
