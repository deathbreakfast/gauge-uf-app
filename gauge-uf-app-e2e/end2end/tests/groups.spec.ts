import { test, expect, seedAuth, waitForHydrated, expectMutationDenied } from "./fixtures";

/** Pick a ComboboxOption from SearchSourcePicker (avoid native `<select>` options). */
async function pickSearchResult(page: import("@playwright/test").Page, label: string) {
  const listbox = page.getByRole("listbox");
  await expect(listbox).toBeVisible({ timeout: 30_000 });
  // Match the title node exactly so /Deployers/ does not hit NestedDeployers.
  const option = listbox
    .getByRole("option")
    .filter({ has: page.getByText(label, { exact: true }) })
    .first();
  await expect(option).toBeAttached({ timeout: 30_000 });
  // Fluent Combobox: Playwright click works when visible; JS click when the
  // option is attached but reports as not visible (owner picker portal).
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


async function rolePrincipalPresent(
  page: import("@playwright/test").Page,
  role: "owner" | "member",
  label: string,
): Promise<boolean> {
  const aria = role === "owner" ? "Open owner actions" : "Open member actions";
  const buttons = page.getByRole("button", { name: new RegExp(`^${aria}$`, "i") });
  const n = await buttons.count();
  for (let i = 0; i < n; i++) {
    // Innermost ancestor that contains exactly one actions button = this row.
    const row = buttons
      .nth(i)
      .locator(`xpath=ancestor::div[count(.//*[@aria-label='${aria}'])=1][1]`);
    if ((await row.getByText(label, { exact: true }).count()) > 0) return true;
  }
  return false;
}

async function expectRolePrincipal(
  page: import("@playwright/test").Page,
  role: "owner" | "member",
  label: string,
) {
  await expect
    .poll(async () => rolePrincipalPresent(page, role, label), { timeout: 60_000 })
    .toBe(true);
}

async function openRoleActions(
  page: import("@playwright/test").Page,
  role: "owner" | "member",
  label: string,
) {
  const aria = role === "owner" ? "Open owner actions" : "Open member actions";
  await expect
    .poll(async () => rolePrincipalPresent(page, role, label), { timeout: 60_000 })
    .toBe(true);
  const buttons = page.getByRole("button", { name: new RegExp(`^${aria}$`, "i") });
  const n = await buttons.count();
  for (let i = 0; i < n; i++) {
    const row = buttons
      .nth(i)
      .locator(`xpath=ancestor::div[count(.//*[@aria-label='${aria}'])=1][1]`);
    if ((await row.getByText(label, { exact: true }).count()) > 0) {
      await buttons.nth(i).click();
      return;
    }
  }
  throw new Error(`openRoleActions: no ${role} row for ${label}`);
}


test.describe("e2e.group.index", () => {
  test("e2e.group.index.load_happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/groups", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-groups-index")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText("Deployers", { exact: true })).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.group.index.search_match", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/groups", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByPlaceholder(/Search by group/i).fill("Deployers");
    await expect(page.getByText("Deployers", { exact: true })).toBeVisible({ timeout: 30_000 });
    await expect(page.getByText("Super User", { exact: true })).toHaveCount(0);
  });

  test("e2e.group.index.search_no_match", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/groups", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByPlaceholder(/Search by group/i).fill("zz-no-group");
    await expect(page.getByText(/No groups found/i)).toBeVisible({ timeout: 30_000 });
  });

  test("e2e.group.index.create_cta", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/groups", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByTestId("gauge-groups-index").getByRole("link", { name: /Create Group/i }).click();
    await expect(page).toHaveURL(/\/permission\/create-group/);
  });

  test("e2e.group.index.open_row", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto("/permission/groups", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.locator(`a[href="/permission/groups/${fixtures.group_id}"]`).click();
    await expect(page).toHaveURL(new RegExp(`/permission/groups/${fixtures.group_id}`));
  });
});

test.describe("e2e.group.create", () => {
  test("e2e.group.create.happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-group", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `E2E-Group-${Date.now()}`;
    await page.getByLabel(/Display name/i).fill(name);
    await page.getByLabel(/Description/i).fill("e2e group");
    await page.getByRole("button", { name: /Create Group/i }).click();
    await expect(page).toHaveURL(/\/permission\/groups\//, { timeout: 60_000 });
  });

  test("e2e.group.create.no_admin", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/permission/create-group", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByLabel(/Display name/i).fill("BlockedGroup");
    await page.getByRole("button", { name: /Create Group/i }).click();
    await expectMutationDenied(page);
  });

  test("e2e.group.create.empty_name", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-group", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByRole("button", { name: /Create Group/i }).click();
    // Server-side reject surfaces as MessageBar (no client empty-name gate yet)
    await expect(page.locator(".orbital-message-bar--error").first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.group.create.dup_name", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-group", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByLabel(/Display name/i).fill("Deployers");
    await page.getByLabel(/Description/i).fill("dup");
    await page.getByRole("button", { name: /Create Group/i }).click();
    await expect(page.locator(".orbital-message-bar--error").first()).toBeVisible({
      timeout: 60_000,
    });
  });
});

test.describe("e2e.group.detail", () => {
  test("e2e.group.detail.load_happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByLabel(/^Name$/i)).toHaveValue("Deployers", { timeout: 60_000 });
  });

  test("e2e.group.detail.add_member_user", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    if (!(await rolePrincipalPresent(page, "member", "requestor"))) {
      const picker = page.getByPlaceholder(/Search users or groups/i).last();
      await picker.click();
      await page.keyboard.type("requestor");
      await pickSearchResult(page, "requestor");
    }
    await expectRolePrincipal(page, "member", "requestor");
  });

  test("e2e.group.detail.add_member_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    if (!(await rolePrincipalPresent(page, "member", "NestedDeployers"))) {
      const picker = page.getByPlaceholder(/Search users or groups/i).last();
      await picker.click();
      await page.keyboard.type("NestedDeployers");
      await pickSearchResult(page, "NestedDeployers");
    }
    await expectRolePrincipal(page, "member", "NestedDeployers");
  });

  test("e2e.group.detail.add_owner", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    if (!(await rolePrincipalPresent(page, "owner", "requestor"))) {
      const picker = page.getByPlaceholder(/Search users or groups/i).first();
      await picker.click();
      await page.keyboard.type("requestor");
      await pickSearchResult(page, "requestor");
    }
    await expectRolePrincipal(page, "owner", "requestor");
  });

  test("e2e.group.detail.remove_owner", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    if (!(await rolePrincipalPresent(page, "owner", "requestor"))) {
      const picker = page.getByPlaceholder(/Search users or groups/i).first();
      await picker.click();
      await page.keyboard.type("requestor");
      await pickSearchResult(page, "requestor");
      await expectRolePrincipal(page, "owner", "requestor");
    }
    await openRoleActions(page, "owner", "requestor");
    await page.getByRole("menuitem", { name: /Remove Owner/i }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    await dialog.getByRole("button", { name: /Remove Owner/i }).click();
    await expect(dialog).toHaveCount(0, { timeout: 60_000 });
    await expect
      .poll(async () => rolePrincipalPresent(page, "owner", "requestor"), { timeout: 60_000 })
      .toBe(false);
  });

  test("e2e.group.detail.owner_picker_no_admin", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
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

  test("e2e.group.detail.member_picker_no_admin", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const picker = page.getByPlaceholder(/Search users or groups/i).last();
    await expect(picker).toBeVisible({ timeout: 60_000 });
    await picker.click();
    await page.keyboard.type("admin");
    await expect(page.locator(".orbital-message-bar--error").first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.group.detail.save_no_admin", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const save = page.getByRole("button", { name: /Save Changes/i });
    await expect(save).toBeVisible({ timeout: 60_000 });
    await page.getByLabel(/Description/i).first().fill("blocked");
    await save.click();
    await expectMutationDenied(page);
  });

  test("e2e.group.detail.save_happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const desc = `group-desc-${Date.now()}`;
    await page.getByLabel(/Description/i).first().fill(desc);
    await page.getByRole("button", { name: /Save Changes/i }).click();
    await expect(page.getByLabel(/Description/i).first()).toHaveValue(desc, { timeout: 60_000 });
  });

  test("e2e.group.detail.delete_happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-group", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `E2E-Del-Group-${Date.now()}`;
    await page.getByLabel(/Display name|Group name/i).fill(name);
    await page.getByLabel(/Description/i).fill("delete me");
    await page.getByRole("button", { name: /Create Group/i }).click();
    await expect(page).toHaveURL(/\/permission\/groups\//, { timeout: 60_000 });
    await waitForHydrated(page);
    await page.getByRole("button", { name: /Delete Group/i }).click();
    await expect(page).toHaveURL(/\/permission\/groups\/?$/, { timeout: 60_000 });
    await waitForHydrated(page);
    await page.goto("/permission/groups", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByPlaceholder(/Search by group/i).fill(name);
    await expect(
      page.locator("#gauge-groups-list").getByText(name, { exact: true }),
    ).toHaveCount(0, { timeout: 30_000 });
  });

  test("e2e.group.detail.request_submit_happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const requestBtn = page.getByRole("button", { name: /Request Access/i });
    await expect(requestBtn).toBeVisible({ timeout: 60_000 });
    await requestBtn.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    await dialog.locator("textarea").fill(`want group ${Date.now()}`);
    await dialog.getByRole("button", { name: "Submit Request", exact: true }).click();
    await expect(dialog).toHaveCount(0, { timeout: 60_000 });
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText(/PENDING/i).first()).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText(/Deployers/i).first()).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.group.detail.request_btn_hidden", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const memberActions = page.getByRole("button", { name: /Open member actions/i });
    let hasRequestor = false;
    const n = await memberActions.count();
    for (let i = 0; i < n; i++) {
      const text = await memberActions
        .nth(i)
        .locator("xpath=ancestor::div[1]")
        .innerText()
        .catch(() => "");
      if (text.includes("requestor")) {
        hasRequestor = true;
        break;
      }
    }
    if (!hasRequestor) {
      const memberPicker = page.getByPlaceholder(/Search users or groups/i).last();
      await memberPicker.click();
      await page.keyboard.type("requestor");
      await pickSearchResult(page, "requestor");
      await expectRolePrincipal(page, "member", "requestor");
    }
    await seedAuth(page, "requestor", false);
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByRole("button", { name: /Request Access/i })).toHaveCount(0);
  });

  test("e2e.group.detail.request_empty_reason", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await page.getByRole("button", { name: /Request Access/i }).click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: "Submit Request", exact: true }).click();
    await expect(page.getByText(/Reason is required/i)).toBeVisible({ timeout: 30_000 });
  });

  test("e2e.group.detail.not_found", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/groups/does-not-exist-xyz", {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByText(/Group not found/i).first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.group.detail.history_entries", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const stamp = `ghist-${Date.now()}`;
    await page.getByLabel(/Description/i).first().fill(stamp);
    await page.getByRole("button", { name: /Save Changes/i }).click();
    await page.getByRole("button", { name: /Show History|View History|History/i }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 30_000 });
    await expect(dialog.getByText(/No history entries yet/i)).toHaveCount(0);
    await expect(dialog.getByText(/Loading history/i)).toHaveCount(0);
    await expect(dialog.getByTestId("record-history-timeline")).toBeVisible({
      timeout: 60_000,
    });
    await expect(dialog.getByText(stamp).first()).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.group.detail.history_relation_member", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-group", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `E2E-HistRel-${Date.now()}`;
    await page.getByLabel(/Display name|Name/i).first().fill(name);
    await page.getByLabel(/Description/i).fill("relation history");
    await page.getByRole("button", { name: /Create Group/i }).click();
    await expect(page).toHaveURL(/\/permission\/groups\//, { timeout: 60_000 });
    await waitForHydrated(page);
    const picker = page.getByPlaceholder(/Search users or groups/i).last();
    await picker.click();
    await page.keyboard.type("requestor");
    await pickSearchResult(page, "requestor");
    await expectRolePrincipal(page, "member", "requestor");
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

  test("e2e.group.detail.history_acl_deny", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
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
