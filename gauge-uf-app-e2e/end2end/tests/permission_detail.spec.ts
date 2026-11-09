import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  expectMutationDenied,
  PEOPLE,
  pickPrincipal,
  pickerRow,
  searchPickerAnswered,
  userRow,
} from "./fixtures";

/** A principal the allow list can hold: how to find it in the picker and in the list. */
type Grantee = { query: string; name: string; exact: boolean; listed: string | RegExp };

const REQUESTOR: Grantee = {
  query: "Rae",
  name: PEOPLE.requestor,
  exact: false,
  listed: userRow("requestor"),
};
const DEPLOYERS: Grantee = { query: "Deployers", name: "Deployers", exact: true, listed: "Deployers" };

function textIn(row: import("@playwright/test").Locator, label: string | RegExp) {
  return typeof label === "string" ? row.getByText(label, { exact: true }) : row.getByText(label);
}

async function principalHasActions(
  page: import("@playwright/test").Page,
  label: string | RegExp,
): Promise<boolean> {
  const aria = "Open member actions";
  const buttons = page.getByRole("button", { name: new RegExp(`^${aria}$`, "i") });
  const n = await buttons.count();
  for (let i = 0; i < n; i++) {
    const row = buttons
      .nth(i)
      .locator(`xpath=ancestor::div[count(.//*[@aria-label='${aria}'])=1][1]`);
    if ((await textIn(row, label).count()) > 0) return true;
  }
  return false;
}


async function expectGrantedPrincipal(
  page: import("@playwright/test").Page,
  label: string | RegExp,
) {
  await expect
    .poll(async () => principalHasActions(page, label), { timeout: 60_000 })
    .toBe(true);
}

async function expectPrincipalRevoked(
  page: import("@playwright/test").Page,
  label: string | RegExp,
) {
  await expect
    .poll(async () => principalHasActions(page, label), { timeout: 60_000 })
    .toBe(false);
}

async function ensureGrantedPrincipal(page: import("@playwright/test").Page, who: Grantee) {
  if (await principalHasActions(page, who.listed)) return;
  await pickPrincipal(page, "gauge-perm-allow-picker", who.query, who.name, who.exact);
  await expectGrantedPrincipal(page, who.listed);
}

async function revokePrincipal(
  page: import("@playwright/test").Page,
  label: string | RegExp,
  confirm: boolean,
) {
  await expect
    .poll(async () => principalHasActions(page, label), { timeout: 60_000 })
    .toBe(true);
  const aria = "Open member actions";
  const buttons = page.getByRole("button", { name: new RegExp(`^${aria}$`, "i") });
  const n = await buttons.count();
  let clicked = false;
  for (let i = 0; i < n; i++) {
    const row = buttons
      .nth(i)
      .locator(`xpath=ancestor::div[count(.//*[@aria-label='${aria}'])=1][1]`);
    if ((await textIn(row, label).count()) > 0) {
      await buttons.nth(i).click();
      clicked = true;
      break;
    }
  }
  if (!clicked) throw new Error(`revokePrincipal: no allow-list row for ${label}`);
  await page.getByRole("menuitem", { name: "Remove", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible({ timeout: 15_000 });
  if (confirm) {
    await dialog.getByRole("button", { name: "Remove", exact: true }).click();
    await expect(dialog).toHaveCount(0, { timeout: 60_000 });
  } else {
    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(dialog).toHaveCount(0, { timeout: 15_000 });
  }
}


test.describe("e2e.perm.detail", () => {
  test("e2e.perm.detail.load_happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByLabel(/^Name$/i)).toHaveValue("CanDeploy", { timeout: 60_000 });
  });

  test("e2e.perm.detail.allow_empty", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `E2E-EmptyAllow-${Date.now()}`;
    await page.getByLabel(/Display name/i).fill(name);
    await page.getByLabel(/Description/i).fill("empty allow list");
    await page.locator("select").first().selectOption({ label: "Ops" });
    await page.getByRole("button", { name: /Create Permission/i }).click();
    await expect(page).toHaveURL(/\/permission\/permissions\//, { timeout: 60_000 });
    await waitForHydrated(page);
    await expect(
      page.getByText(/No principals are currently in the allow list/i),
    ).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.perm.detail.save_happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const desc = page.getByLabel(/Description/i).first();
    await desc.fill(`updated-desc-${Date.now()}`);
    const domainSelect = page.locator("select").first();
    await expect(domainSelect).toBeVisible({ timeout: 30_000 });
    await domainSelect.selectOption({ label: "Platform" });
    await page.getByRole("button", { name: /Save Changes/i }).click();
    await expect(domainSelect.locator("option:checked")).toHaveText(/Platform/i, {
      timeout: 60_000,
    });
  });

  test("e2e.perm.detail.save_no_admin", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const save = page.getByRole("button", { name: /Save Changes/i });
    await expect(save).toBeVisible({ timeout: 60_000 });
    await page.getByLabel(/Description/i).first().fill("should fail");
    await save.click();
    await expectMutationDenied(page);
  });

  test("e2e.perm.detail.grant_user", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, REQUESTOR);
  });

  test("e2e.perm.detail.grant_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, DEPLOYERS);
  });

  test("e2e.perm.detail.picker_search", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const list = await searchPickerAnswered(page, "gauge-perm-allow-picker", "Rae");
    const rae = pickerRow(list, PEOPLE.requestor);
    await expect(rae).toBeVisible({ timeout: 30_000 });
    await expect(rae).toHaveAttribute("data-kind", "user");
    await expect(rae).not.toContainText("@");
    await expect(pickerRow(list, PEOPLE.admin)).toHaveCount(0);
  });

  test("e2e.perm.detail.revoke_user", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, REQUESTOR);
    await revokePrincipal(page, REQUESTOR.listed, true);
    await expectPrincipalRevoked(page, REQUESTOR.listed);
  });

  test("e2e.perm.detail.revoke_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, DEPLOYERS);
    await revokePrincipal(page, DEPLOYERS.listed, true);
    await expectPrincipalRevoked(page, DEPLOYERS.listed);
  });

  test("e2e.perm.detail.revoke_cancel", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, REQUESTOR);
    await revokePrincipal(page, REQUESTOR.listed, false);
    await expectGrantedPrincipal(page, REQUESTOR.listed);
  });

  test("e2e.perm.detail.request_btn_hidden", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, REQUESTOR);
    await seedAuth(page, "requestor", false);
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByRole("button", { name: /Request Access/i })).toHaveCount(0);
  });

  test("e2e.perm.detail.picker_no_admin", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await pickPrincipal(page, "gauge-perm-allow-picker", "Otto", PEOPLE.outsider);
    await expectMutationDenied(page);
    await expect.poll(async () => principalHasActions(page, userRow("outsider"))).toBe(false);
  });

  test("e2e.perm.detail.request_btn_visible", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByRole("button", { name: /Request Access/i })).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.perm.detail.request_submit_happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const requestBtn = page.getByRole("button", { name: /Request Access/i });
    await expect(requestBtn).toBeVisible({ timeout: 60_000 });
    await requestBtn.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    const reason = `outsider needs access ${Date.now()}`;
    await dialog.locator("textarea").fill(reason);
    await dialog.getByRole("button", { name: "Submit Request", exact: true }).click();
    await expect(dialog).toHaveCount(0, { timeout: 60_000 });
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    // Scoped to the list: the Status filter also has a "Pending" option.
    const mine = page.locator("#gauge-requests-mine");
    await expect(mine.getByText(/PENDING/).first()).toBeVisible({ timeout: 60_000 });
    await expect(mine.getByText(/CanDeploy/i).first()).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.perm.detail.request_empty_reason", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const requestBtn = page.getByRole("button", { name: /Request Access/i });
    await expect(requestBtn).toBeVisible({ timeout: 60_000 });
    await requestBtn.click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: "Submit Request", exact: true }).click();
    await expect(page.getByText(/Reason is required/i)).toBeVisible({ timeout: 30_000 });
  });

  test("e2e.perm.detail.not_found", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/permissions/does-not-exist-xyz", {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByText(/Permission not found/i)).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.perm.detail.delete_happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `E2E-Del-${Date.now()}`;
    await page.getByLabel(/Display name/i).fill(name);
    await page.getByLabel(/Description/i).fill("delete me");
    const domainSelect = page.locator("select").first();
    await domainSelect.selectOption({ label: "Ops" });
    await page.getByRole("button", { name: /Create Permission/i }).click();
    await expect(page).toHaveURL(/\/permission\/permissions\//, { timeout: 60_000 });
    await waitForHydrated(page);
    await page.getByRole("button", { name: /Delete Permission/i }).click();
    // Must leave the detail URL — `/permission` alone also matches detail paths.
    await expect(page).toHaveURL(/\/permission\/permissions\/?$/, { timeout: 60_000 });
    await waitForHydrated(page);
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByPlaceholder(/Search by permission/i).fill(name);
    await expect(
      page.locator("#gauge-permissions-list").getByText(name, { exact: true }),
    ).toHaveCount(0, { timeout: 30_000 });
  });

  test("e2e.perm.detail.history_entries", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const stamp = `hist-${Date.now()}`;
    await page.getByLabel(/Description/i).first().fill(stamp);
    await page.getByRole("button", { name: /Save Changes/i }).click();
    await page.getByRole("button", { name: /Show History|View History|History/i }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 30_000 });
    await expect(dialog.getByText(/No history entries yet/i)).toHaveCount(0);
    await expect(dialog.getByText(/Loading history/i)).toHaveCount(0);
    // Durable timeline + stamp — not DialogTitle "History" / loading chrome.
    await expect(dialog.getByTestId("record-history-timeline")).toBeVisible({
      timeout: 60_000,
    });
    await expect(dialog.getByText(stamp).first()).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.perm.detail.history_relation_grant", async ({ page }) => {
    // Seeding, the grant and the 90 s retry window below all share this budget.
    test.setTimeout(150_000);
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, REQUESTOR);
    // The dialog loads history once on open and the relation entry can land
    // after the grant returns, so reopen it until an Added/requestor row shows.
    const dialog = page.getByRole("dialog");
    await expect(async () => {
      await page.getByRole("button", { name: /Show History|View History|History/i }).click();
      await expect(dialog).toBeVisible({ timeout: 15_000 });
      await expect(dialog.getByTestId("record-history-timeline")).toBeVisible({
        timeout: 15_000,
      });
      const added = dialog
        .getByTestId("gauge-history-relation-row")
        .filter({ has: page.getByText(/^Added$/) })
        .filter({ hasText: /requestor/i });
      try {
        await expect.poll(async () => added.count(), { timeout: 5_000 }).toBeGreaterThan(0);
      } finally {
        await dialog.getByRole("button", { name: "Close", exact: true }).click();
        await expect(dialog).toHaveCount(0, { timeout: 15_000 });
      }
    }).toPass({ timeout: 90_000, intervals: [1_000, 2_000, 5_000] });
  });

  test("e2e.perm.detail.history_acl_deny", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await page.getByRole("button", { name: /Show History|View History|History/i }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 30_000 });
    await expect(dialog.getByTestId("gauge-history-access-denied")).toBeVisible({
      timeout: 60_000,
    });
    await expect(
      dialog.getByText(/Not authorized to view this history/i),
    ).toBeVisible();
    await expect(dialog.getByTestId("record-history-timeline")).toBeHidden();
  });
});
