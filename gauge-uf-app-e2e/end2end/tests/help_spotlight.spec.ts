import { test, expect, seedAuth, waitForHydrated } from "./fixtures";
import type { Page } from "@playwright/test";

async function completeVisibleTour(page: Page) {
  const footer = page.locator('[data-testid="spotlight-footer"]:visible');
  const next = footer.getByTestId("spotlight-tour-next");
  await expect(footer).toBeVisible({ timeout: 60_000 });
  for (let i = 0; i < 24; i++) {
    if ((await footer.count()) === 0) {
      break;
    }
    // Spotlight panels can sit partially off-screen; DOM click avoids Playwright
    // viewport hit-testing failures that still occur with { force: true }.
    await next.evaluate((el: HTMLElement) => el.click());
    try {
      await expect(footer).toHaveCount(0, { timeout: 2_000 });
      break;
    } catch {
      /* more steps */
    }
  }
  await expect(footer).toHaveCount(0, { timeout: 30_000 });
}

test.describe("help-spotlight", () => {
  test("help-spotlight-skips-when-seeded", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-overview")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("help-step-overview-intro")).toHaveCount(0);
    await expect(page.locator('[data-testid="spotlight-footer"]:visible')).toHaveCount(0);
  });

  test("help-spotlight-skips-auth-gate", async ({ page }) => {
    await seedAuth(page, "anonymous", false, { help_tour: true });
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("auth-required-empty-state")).toBeAttached({
      timeout: 60_000,
    });
    await expect(page.getByTestId("help-step-overview-intro")).toHaveCount(0);
    await expect(page.locator('[data-testid="spotlight-footer"]:visible')).toHaveCount(0);
  });

  test("help-spotlight-overview-green", async ({ page }) => {
    await seedAuth(page, "admin", false, { help_tour: true });
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-overview-intro")).toBeVisible({ timeout: 60_000 });
    await completeVisibleTour(page);
  });

  test("help-spotlight-people-green", async ({ page }) => {
    await seedAuth(page, "admin", false, { help_tour: true });
    await page.goto("/permission/people", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-people-intro")).toBeVisible({ timeout: 60_000 });
    await completeVisibleTour(page);
  });

  test("help-spotlight-activity-green", async ({ page }) => {
    await seedAuth(page, "admin", false, { help_tour: true });
    await page.goto("/permission/activity", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-activity-filters")).toBeVisible({ timeout: 60_000 });
    await completeVisibleTour(page);
  });

  test("help-spotlight-permissions-index-green", async ({ page }) => {
    await seedAuth(page, "admin", false, { help_tour: true });
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-permission-intro")).toBeVisible({ timeout: 60_000 });
    await completeVisibleTour(page);
    await expect(page.getByTestId("help-step-permission-intro")).toHaveCount(0);
    // Let help_mark_steps_seen finish before reload — avoids racing tour teardown.
    await page.waitForLoadState("networkidle", { timeout: 30_000 }).catch(() => undefined);

    await page.reload({ waitUntil: "load" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-permission-intro")).toHaveCount(0);
  });

  test("help-spotlight-create-permission-green", async ({ page }) => {
    await seedAuth(page, "admin", false, { help_tour: true });
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-create-permission-intro")).toBeVisible({
      timeout: 60_000,
    });
    await completeVisibleTour(page);
  });

  test("help-spotlight-permission-detail-green", async ({ page }) => {
    const seeded = await seedAuth(page, "admin", false, { help_tour: true });
    const id = seeded.fixtures.permission_id;
    await page.goto(`/permission/permissions/${encodeURIComponent(id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-permission-detail-intro")).toBeVisible({
      timeout: 60_000,
    });
    await completeVisibleTour(page);
  });

  test("help-spotlight-create-domain-green", async ({ page }) => {
    await seedAuth(page, "admin", false, { help_tour: true });
    await page.goto("/permission/create-domain", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-create-domain-intro")).toBeVisible({
      timeout: 60_000,
    });
    await completeVisibleTour(page);
  });

  test("help-spotlight-groups-index-green", async ({ page }) => {
    await seedAuth(page, "admin", false, { help_tour: true });
    await page.goto("/permission/groups", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-groups-intro")).toBeVisible({
      timeout: 60_000,
    });
    await completeVisibleTour(page);
  });

  test("help-spotlight-create-group-green", async ({ page }) => {
    await seedAuth(page, "admin", false, { help_tour: true });
    await page.goto("/permission/create-group", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-create-group-intro")).toBeVisible({
      timeout: 60_000,
    });
    await completeVisibleTour(page);
  });

  test("help-spotlight-group-detail-green", async ({ page }) => {
    const seeded = await seedAuth(page, "admin", false, { help_tour: true });
    const id = seeded.fixtures.group_id;
    await page.goto(`/permission/groups/${encodeURIComponent(id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-group-detail-intro")).toBeVisible({
      timeout: 60_000,
    });
    await completeVisibleTour(page);
  });

  test("help-spotlight-requests-index-green", async ({ page }) => {
    await seedAuth(page, "admin", true, { help_tour: true });
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-requests-intro")).toBeVisible({
      timeout: 60_000,
    });
    await completeVisibleTour(page);
  });

  test("help-spotlight-request-detail-green", async ({ page }) => {
    const seeded = await seedAuth(page, "admin", true, { help_tour: true });
    const id = seeded.fixtures.pending_perm_request_id;
    await page.goto(`/permission/requests/${encodeURIComponent(id)}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("help-step-request-detail-intro")).toBeVisible({
      timeout: 60_000,
    });
    await completeVisibleTour(page);
  });
});
