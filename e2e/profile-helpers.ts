import { expect, type Page } from "@playwright/test";

const profileKeys = [
  "rm-selected-profile",
  "rm-profiles",
  "rm-catalog-cache",
  "rm-tag-cache",
  "rm-audit-events",
];

const mockKeys = [
  "rm-mock-catalog",
  "rm-mock-delete-404",
  "rm-mock-docker-unavailable",
  "rm-mock-gc",
  "rm-mock-gc-failure",
  "rm-mock-registry-offline",
];

interface ManualProfileOptions {
  name?: string;
  url?: string;
  catalog?: boolean;
  gc?: boolean;
}

export async function resetMockState(page: Page) {
  await page.goto("/");
  await page.evaluate(
    ({ profileKeys, mockKeys }) => {
      [...profileKeys, ...mockKeys].forEach((key) => localStorage.removeItem(key));
    },
    { profileKeys, mockKeys }
  );
  await page.reload();
}

export async function addAndSelectManualProfile(page: Page, options: ManualProfileOptions = {}) {
  const name = options.name ?? "Local registry";
  const url = options.url ?? "http://localhost:5000";

  await resetMockState(page);
  await page.evaluate(
    ({ catalog, gc }) => {
      if (catalog) localStorage.setItem("rm-mock-catalog", "true");
      if (gc) localStorage.setItem("rm-mock-gc", "true");
    },
    { catalog: Boolean(options.catalog), gc: Boolean(options.gc) }
  );
  await page.reload();

  await page.getByTestId("rm-add-profile-button").click();
  await page.getByTestId("rm-profile-name-input").fill(name);
  await page.getByTestId("rm-profile-url-input").fill(url);
  await page.getByTestId("rm-profile-save-button").click();

  if (options.gc) {
    await page.evaluate(
      ({ name, url }) => {
        const profiles = JSON.parse(localStorage.getItem("rm-profiles") ?? "[]") as Array<Record<string, unknown>>;
        const updated = profiles.map((profile) => (
          profile.name === name && profile.registryUrl === url
            ? { ...profile, containerId: "mock-registry-container", containerName: "registry" }
            : profile
        ));
        localStorage.setItem("rm-profiles", JSON.stringify(updated));
      },
      { name, url }
    );
    await page.reload();
  }

  const item = page.getByTestId("rm-profile-item").filter({ hasText: name });
  await expect(item).toContainText(url);
  await item.locator("label").click();
  await expect(page.getByTestId("rm-profile-radio").first()).toBeChecked();
  return { name, url };
}
