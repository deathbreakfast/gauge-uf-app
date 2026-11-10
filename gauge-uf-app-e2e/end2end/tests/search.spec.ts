import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  PEOPLE,
  pickerInput,
  pickerList,
  pickerRow,
  searchPickerAnswered,
} from "./fixtures";

const PICKER = "gauge-perm-allow-picker";

test.describe("e2e.search.principals", () => {
  test("e2e.search.principals_initial", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    // An empty box lists the permission groups, never the people directory.
    await pickerInput(page, PICKER).click();
    const list = await pickerList(page, PICKER);
    await expect(list.locator('[data-testid="person-picker-row"][data-kind="extra"]').first())
      .toBeVisible({ timeout: 30_000 });
    await expect(list.locator('[data-testid="person-picker-row"][data-kind="user"]')).toHaveCount(0);
  });

  test("e2e.search.principals_query_user", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const list = await searchPickerAnswered(page, PICKER, "requ");
    const rae = pickerRow(list, PEOPLE.requestor);
    await expect(rae).toBeVisible({ timeout: 30_000 });
    await expect(rae).toHaveAttribute("data-kind", "user");
    await expect(rae).not.toContainText("@");
  });

  test("e2e.search.principals_query_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const list = await searchPickerAnswered(page, PICKER, "Deployers");
    const group = pickerRow(list, "Deployers", true).first();
    await expect(group).toBeVisible({ timeout: 30_000 });
    await expect(group).toHaveAttribute("data-kind", "extra");
  });

  test("e2e.search.principals_no_admin", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    // The group list comes from a GaugeAdmin-only search, so a non-admin sees no groups.
    const list = await searchPickerAnswered(page, PICKER, "Deployers");
    await expect(list.locator('[data-testid="person-picker-row"][data-kind="extra"]')).toHaveCount(0);
  });
});
