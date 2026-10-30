import type { Page } from "@playwright/test";
import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

/** Seeded by gauge's service user boot seed; `requestor` is not a member. */
const CREATORS = "Service user creators";

async function openCreatorsRequest(page: Page, forHandle: string) {
  await page.goto(
    `/permission/groups?name=${encodeURIComponent(CREATORS)}&request=1&for=${encodeURIComponent(forHandle)}`,
    { waitUntil: "domcontentloaded" },
  );
  await waitForHydrated(page);
  await expect(page).toHaveURL(/\/permission\/groups\/[^/?]+\?request=1/, { timeout: 60_000 });
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible({ timeout: 60_000 });
  return dialog;
}

/** File a request for "Service user creators" on behalf of `handle` through the dialog. */
async function requestForServiceUser(page: Page, handle: string) {
  const dialog = await openCreatorsRequest(page, handle);
  await expect(dialog.locator("#gauge-request-for select")).toHaveValue(handle, {
    timeout: 60_000,
  });
  await dialog.locator("textarea").fill(`pw service user request ${Date.now()}`);
  await dialog.getByRole("button", { name: "Submit Request", exact: true }).click();
  await expect(dialog).toHaveCount(0, { timeout: 60_000 });
}

test.describe("gauge.open-by-name", () => {
  test("pw-gauge-group-open-by-name-happy", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto(`/permission/groups?name=${encodeURIComponent(CREATORS.toLowerCase())}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page).toHaveURL(/\/permission\/groups\/[^/?]+$/, { timeout: 60_000 });
    await expect(page.getByRole("button", { name: "Request Access", exact: true })).toBeVisible({
      timeout: 60_000,
    });
  });

  test("pw-gauge-group-open-by-name-missing-sad", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/permission/groups?name=No%20such%20group", {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByText("No group is named No such group.")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page).toHaveURL(/\/permission\/groups\?q=No/);
    await expect(page.getByPlaceholder(/Search by group/i)).toHaveValue("No such group");
  });

  test("pw-gauge-permission-open-by-name-happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto(
      `/permission/permissions?name=${encodeURIComponent(fixtures.permission_name.toUpperCase())}`,
      { waitUntil: "domcontentloaded" },
    );
    await waitForHydrated(page);
    await expect(page).toHaveURL(
      new RegExp(`/permission/permissions/${fixtures.permission_id}$`),
      { timeout: 60_000 },
    );
  });

  test("pw-gauge-groups-search-in-url-happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/groups", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByPlaceholder(/Search by group/i).fill("Deployers");
    await expect(page).toHaveURL(/\/permission\/groups\?q=Deployers$/, { timeout: 60_000 });

    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByPlaceholder(/Search by group/i)).toHaveValue("Deployers", {
      timeout: 60_000,
    });
    const list = page.locator("#gauge-groups-list");
    await expect(list.getByText("Deployers", { exact: true })).toBeVisible({ timeout: 60_000 });
    await expect(list.getByText(CREATORS, { exact: true })).toHaveCount(0);
  });
});

test.describe("gauge.request-for-service-user", () => {
  test("pw-gauge-request-for-service-user-happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    const handle = fixtures.service_user_handle;
    await requestForServiceUser(page, handle);

    await page.goto(`/permission/requests?q=${encodeURIComponent(handle)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const mine = page.locator("#gauge-requests-mine");
    await expect(mine.getByText(`for @${handle}`).first()).toBeVisible({ timeout: 60_000 });

    await mine.getByRole("button", { name: "Open", exact: true }).first().click();
    await expect(page.getByText(`For: @${handle}`)).toBeVisible({ timeout: 60_000 });
  });

  test("pw-gauge-request-for-foreign-service-user-sad", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    const foreign = fixtures.foreign_service_user_handle;
    const dialog = await openCreatorsRequest(page, foreign);
    await expect(
      dialog.getByText(`@${foreign} isn't one of your active service users.`),
    ).toBeVisible({ timeout: 60_000 });
    await expect(dialog.locator("#gauge-request-for select")).toHaveValue("");
    await expect(dialog.locator(`option[value="${foreign}"]`)).toHaveCount(0);
    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();

    await page.goto(`/permission/requests?q=${encodeURIComponent(foreign)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-requests-index")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText(`for @${foreign}`)).toHaveCount(0);
  });

  test("pw-gauge-requests-filter-happy", async ({ page }) => {
    // Refresh mints plain (people) pending requests beside the service user one.
    const { fixtures } = await seedAuth(page, "requestor", true);
    const handle = fixtures.service_user_handle;
    await requestForServiceUser(page, handle);

    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const mine = page.locator("#gauge-requests-mine");
    const status = page.locator("#gauge-requests-filter-status select");
    const forKind = page.locator("#gauge-requests-filter-for select");
    const forHandle = mine.getByText(`for @${handle}`).first();
    const peopleRow = mine.getByText(/^Permission - PENDING$/).first();
    await expect(forHandle).toBeVisible({ timeout: 60_000 });
    await expect(peopleRow).toBeVisible();

    await status.selectOption("approved");
    await expect(page).toHaveURL(/status=approved/);
    await expect(mine.getByText(`for @${handle}`)).toHaveCount(0);

    await status.selectOption("pending");
    await forKind.selectOption("people");
    await expect(page).toHaveURL(/for=people/);
    await expect(mine.getByText(`for @${handle}`)).toHaveCount(0);
    await expect(peopleRow).toBeVisible();

    await forKind.selectOption("service_users");
    await expect(page).toHaveURL(/status=pending/);
    await expect(page).toHaveURL(/for=service_users/);
    await expect(forHandle).toBeVisible();
    await expect(mine.getByText(/^Permission - PENDING$/)).toHaveCount(0);

    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(status).toHaveValue("pending", { timeout: 60_000 });
    await expect(forKind).toHaveValue("service_users");
    await expect(forHandle).toBeVisible({ timeout: 60_000 });
    await expect(mine.getByText(/^Permission - PENDING$/)).toHaveCount(0);
  });
});
