import { test, expect, seedAuth, waitForHydrated, expectMutationDenied } from "./fixtures";

async function pickSearchResult(page: import("@playwright/test").Page, label: string) {
  const listbox = page.getByRole("listbox");
  await expect(listbox).toBeVisible({ timeout: 30_000 });
  // Match the title node exactly so /Deployers/ does not hit NestedDeployers.
  const option = listbox
    .getByRole("option")
    .filter({ has: page.getByText(label, { exact: true }) })
    .first();
  await expect(option).toBeAttached({ timeout: 30_000 });
  try {
    await option.click({ force: true, timeout: 5_000 });
  } catch {
    await option.evaluate((el: HTMLElement) => {
      el.dispatchEvent(
        new MouseEvent("click", { bubbles: true, cancelable: true, view: window }),
      );
    });
  }
  await page.keyboard.press("Escape").catch(() => undefined);
}

async function typeIntoPicker(
  page: import("@playwright/test").Page,
  label: string,
) {
  const picker = page.getByPlaceholder(/Search users or groups/i).first();
  await picker.click();
  await picker.fill("");
  await page.keyboard.type(label);
}


async function principalHasActions(
  page: import("@playwright/test").Page,
  label: string,
): Promise<boolean> {
  const aria = "Open member actions";
  const buttons = page.getByRole("button", { name: new RegExp(`^${aria}$`, "i") });
  const n = await buttons.count();
  for (let i = 0; i < n; i++) {
    const row = buttons
      .nth(i)
      .locator(`xpath=ancestor::div[count(.//*[@aria-label='${aria}'])=1][1]`);
    if ((await row.getByText(label, { exact: true }).count()) > 0) return true;
  }
  return false;
}


async function expectGrantedPrincipal(
  page: import("@playwright/test").Page,
  label: string,
) {
  await expect
    .poll(async () => principalHasActions(page, label), { timeout: 60_000 })
    .toBe(true);
}

async function expectPrincipalRevoked(
  page: import("@playwright/test").Page,
  label: string,
) {
  await expect
    .poll(async () => principalHasActions(page, label), { timeout: 60_000 })
    .toBe(false);
}

async function ensureGrantedPrincipal(
  page: import("@playwright/test").Page,
  label: string,
) {
  if (await principalHasActions(page, label)) return;
  await typeIntoPicker(page, label);
  await pickSearchResult(page, label);
  await expectGrantedPrincipal(page, label);
}

async function revokePrincipal(page: import("@playwright/test").Page, label: string, confirm: boolean) {
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
    if ((await row.getByText(label, { exact: true }).count()) > 0) {
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
    await ensureGrantedPrincipal(page, "requestor");
  });

  test("e2e.perm.detail.grant_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, "Deployers");
  });

  test("e2e.perm.detail.picker_search", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const picker = page.getByPlaceholder(/Search users or groups/i).first();
    await picker.click();
    await page.keyboard.type("request");
    const listbox = page.getByRole("listbox");
    await expect(listbox.getByRole("option", { name: /requestor/i })).toBeVisible({
      timeout: 30_000,
    });
    await expect(listbox.getByRole("option", { name: /^admin$/i })).toHaveCount(0);
  });

  test("e2e.perm.detail.revoke_user", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, "requestor");
    await revokePrincipal(page, "requestor", true);
    await expectPrincipalRevoked(page, "requestor");
  });

  test("e2e.perm.detail.revoke_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, "Deployers");
    await revokePrincipal(page, "Deployers", true);
    await expectPrincipalRevoked(page, "Deployers");
  });

  test("e2e.perm.detail.revoke_cancel", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, "requestor");
    await revokePrincipal(page, "requestor", false);
    await expectGrantedPrincipal(page, "requestor");
  });

  test("e2e.perm.detail.request_btn_hidden", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, "requestor");
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
    const picker = page.getByPlaceholder(/Search users or groups/i).first();
    await expect(picker).toBeVisible({ timeout: 60_000 });
    await picker.click();
    await page.keyboard.type("admin");
    await expect(page.locator(".orbital-message-bar--error").first()).toBeVisible({
      timeout: 60_000,
    });
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
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await ensureGrantedPrincipal(page, "requestor");
    await page.getByRole("button", { name: /Show History|View History|History/i }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 30_000 });
    await expect(dialog.getByTestId("record-history-timeline")).toBeVisible({
      timeout: 60_000,
    });
    const relation = dialog.getByTestId("gauge-history-relation-row").first();
    await expect(relation).toBeVisible({ timeout: 60_000 });
    await expect(relation.getByText(/^Added$/)).toBeVisible();
    await expect(relation.getByText(/requestor/i).first()).toBeVisible();
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
