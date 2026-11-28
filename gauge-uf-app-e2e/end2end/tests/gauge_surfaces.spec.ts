import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.use({ timezoneId: "UTC" });

test.describe("gauge.overview", () => {
  test("pw-gauge-app-bar-says-gauge-happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-overview")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByRole("heading", { name: "Gauge", exact: true })).toBeVisible();
    await expect(page.getByTestId("nav-people")).toBeAttached({ timeout: 60_000 });
    await expect(page.getByTestId("nav-domains")).toBeAttached();
    await expect(page.getByTestId("nav-activity")).toBeAttached();
    await expect(page.getByTestId("nav-create-permission")).toHaveCount(0);
  });

  test("pw-gauge-member-nav-hides-admin-places-sad", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-overview-access")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("nav-overview")).toBeAttached();
    await expect(page.getByTestId("nav-requests")).toBeAttached();
    await expect(page.getByTestId("nav-people")).toHaveCount(0);
    await expect(page.getByTestId("nav-domains")).toHaveCount(0);
    await expect(page.getByTestId("nav-activity")).toHaveCount(0);
    await expect(page.getByTestId("gauge-overview-stats")).toHaveCount(0);
  });

  test("pw-gauge-overview-lists-waiting-requests-happy", async ({ page }) => {
    await seedAuth(page, "admin", true);
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const waiting = page.locator("#gauge-overview-waiting");
    await expect(waiting.getByTestId("gauge-overview-waiting-row").first()).toBeVisible({
      timeout: 60_000,
    });
    await expect(waiting.getByText(/^Waiting for you \(\d+\)$/)).toBeVisible();
    await expect(waiting.locator("time[datetime]").first()).toBeAttached();
  });
});

test.describe("gauge.people", () => {
  test("pw-gauge-person-lists-permission-with-group-happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/people/requestor", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const row = page
      .getByTestId("gauge-person-permission-row")
      .filter({ hasText: "Approve releases" });
    await expect(row).toBeVisible({ timeout: 60_000 });
    await expect(row.getByText("Through Release Captains")).toBeVisible();
    await expect(page.getByTestId("gauge-person-owns")).toBeVisible();
    await expect(page.getByTestId("gauge-person-requests")).toBeVisible();
  });

  test("pw-gauge-people-service-users-tab-opens-page-happy", async ({ page }) => {
    // Gauge lists only the service users the viewer can see; admin owns admin-agent, which
    // gets a principal row by joining the fixture group.
    const { fixtures } = await seedAuth(page, "admin", false, { serviceUserMember: true });
    await page.goto("/permission/people?tab=service-users", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const table = page.getByTestId("gauge-people-table");
    await expect(table.getByText(fixtures.foreign_service_user_handle, { exact: false }).first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("pw-gauge-people-denied-for-member-sad", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/permission/people", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-denied-page")).toBeVisible({ timeout: 60_000 });
  });

  test("pw-gauge-person-not-found-sad", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/people/nobody-here", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-person-not-found")).toBeVisible({ timeout: 60_000 });
  });
});

test.describe("gauge.activity", () => {
  test("pw-gauge-activity-filters-by-kind-happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/activity", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-activity-table")).toBeVisible({ timeout: 60_000 });
    await page.getByTestId("gauge-activity-filter-kind").locator("select").selectOption("membership");
    await expect(page).toHaveURL(/kind=membership/);
    const list = page.locator("#gauge-activity-list");
    // The phone list view shows the record without its kind caption.
    if (test.info().project.name !== "phone") {
      await expect(list.getByText("Membership", { exact: true }).first()).toBeVisible({
        timeout: 60_000,
      });
    }
    await expect(list.getByText("Allow lists", { exact: true })).toHaveCount(0);
    await expect(list.getByText(/\d{4}-\d{2}-\d{2}T\d{2}:/)).toHaveCount(0);
  });

  test("pw-gauge-activity-denied-for-member-sad", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/permission/activity", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const denied = page.getByTestId("gauge-denied-page");
    await expect(denied.getByText("Activity is for Gauge admins")).toBeVisible({ timeout: 60_000 });
  });
});

test.describe("gauge.domains", () => {
  test("pw-gauge-domains-list-happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/domains", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const table = page.getByTestId("gauge-domains-table");
    await expect(table.getByText("Ops", { exact: true })).toBeVisible({ timeout: 60_000 });
    await expect(page.locator("#gauge-domains-create").getByRole("button")).toBeVisible();
  });

  test("pw-gauge-domain-blocked-delete-names-permissions-sad", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/domains/${fixtures.domain_a_id}`, { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-domain-crumbs")).toBeVisible({ timeout: 60_000 });
    await page.locator("#gauge-domain-delete").getByRole("button").click();
    const blocked = page.getByRole("dialog").getByTestId("gauge-domain-delete-blocked");
    await expect(blocked).toBeVisible({ timeout: 60_000 });
    await expect(blocked.getByText("ApproveRelease", { exact: true })).toBeVisible();
    await expect(page.getByRole("dialog").getByText(/can't be deleted yet$/)).toBeVisible();
  });

  test("pw-gauge-domain-not-found-sad", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/domains/missing-domain", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const missing = page.getByTestId("gauge-domain-not-found");
    await expect(missing).toBeVisible({ timeout: 60_000 });
    await missing.getByRole("button", { name: "Back to Domains" }).click();
    await expect(page).toHaveURL(/\/permission\/domains$/, { timeout: 60_000 });
  });
});

test.describe("gauge.service-user-page", () => {
  test("pw-gauge-service-user-groups-and-through-lines-happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto(`/permission/service-users/${fixtures.service_user_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const header = page.locator("#gauge-service-user-header");
    await expect(header.getByText(new RegExp(`^@${fixtures.service_user_handle} · Owned by `))).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("gauge-service-user-status")).toHaveText("Active");
    await expect(
      page.getByTestId("gauge-service-user-group-row").filter({ hasText: "Release bots" }),
    ).toBeVisible();
    const perm = page
      .getByTestId("gauge-service-user-permission-row")
      .filter({ hasText: "Post release timings" });
    await expect(perm.getByText("Through")).toBeVisible();
    await expect(perm.getByRole("link", { name: "Release bots" })).toBeVisible();
  });

  test("pw-gauge-service-user-paused-sad", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto(`/permission/service-users/${fixtures.paused_service_user_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-service-user-paused")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("gauge-service-user-status")).toHaveText("Paused");
  });

  test("pw-gauge-service-user-not-found-for-others-sad", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await page.goto(`/permission/service-users/${fixtures.service_user_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const missing = page.getByTestId("gauge-service-user-not-found");
    await expect(missing.getByText("Service user not found")).toBeVisible({ timeout: 60_000 });
  });

  test("pw-gauge-group-member-links-service-user-page-happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", false, { serviceUserMember: true });
    await page.goto(`/permission/groups/${fixtures.group_id}`, { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const link = page
      .getByTestId("gauge-group-members")
      .locator(`a[href="/permission/service-users/${fixtures.foreign_service_user_id}"]`);
    await expect(link).toBeVisible({ timeout: 60_000 });
  });
});
