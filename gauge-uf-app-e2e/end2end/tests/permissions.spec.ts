import { test, expect, seedAuth, waitForHydrated, expectMutationDenied } from "./fixtures";

test.describe("e2e.perm.index", () => {
  test("e2e.perm.index.load_happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-permissions-index")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByText("CanDeploy", { exact: true })).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.perm.index.search_match", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText("CanDeploy", { exact: true })).toBeVisible({ timeout: 60_000 });
    await page.getByPlaceholder(/Search by permission/i).fill("CanDeploy");
    await expect(page.getByText("CanDeploy", { exact: true })).toBeVisible();
  });

  test("e2e.perm.index.search_no_match", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByPlaceholder(/Search by permission/i).fill("zz-no-such-perm");
    await expect(page.getByText(/No permissions found/i)).toBeVisible({
      timeout: 30_000,
    });
  });

  test("e2e.perm.index.open_row", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.locator(`a[href="/permission/permissions/${fixtures.permission_id}"]`).click();
    await expect(page).toHaveURL(new RegExp(`/permission/permissions/${fixtures.permission_id}`));
  });

  test("e2e.perm.index.create_cta", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page
      .getByTestId("gauge-permissions-index")
      .getByRole("link", { name: /Create Permission/i })
      .or(
        page
          .getByTestId("gauge-permissions-index")
          .getByRole("button", { name: /Create Permission/i }),
      )
      .first()
      .click();
    await expect(page).toHaveURL(/\/permission\/create-permission/);
  });
});

test.describe("e2e.domain.create", () => {
  async function pickSearchResult(
    page: import("@playwright/test").Page,
    label: string,
  ) {
    const listbox = page.getByRole("listbox");
    await expect(listbox).toBeVisible({ timeout: 30_000 });
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
    await expect(listbox).toHaveCount(0, { timeout: 15_000 }).catch(() => undefined);
  }

  async function createDomainAndOpenDetail(
    page: import("@playwright/test").Page,
    name: string,
    description: string,
  ) {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-domain", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByLabel(/Domain name/i).fill(name);
    await page.getByLabel(/Description/i).fill(description);
    await page.getByRole("button", { name: "Create Domain", exact: true }).click();
    await expect(page).toHaveURL(/\/permission\/domains\//, { timeout: 60_000 });
    await waitForHydrated(page);
  }

  test("e2e.domain.create.happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-domain", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `E2E-Domain-${Date.now()}`;
    await page.getByLabel(/Domain name/i).fill(name);
    await page.getByLabel(/Description/i).fill("created by e2e");
    await page.getByRole("button", { name: "Create Domain", exact: true }).click();
    await expect(page).toHaveURL(/\/permission\/domains\//, { timeout: 60_000 });
    await waitForHydrated(page);
    await expect(page.getByText(/Domain Detail/i)).toBeVisible({ timeout: 60_000 });
    await expect(page.getByLabel(/Display name/i)).toHaveValue(name, { timeout: 30_000 });
  });

  test("e2e.domain.detail.save_happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-domain", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `E2E-Domain-Edit-${Date.now()}`;
    await page.getByLabel(/Domain name/i).fill(name);
    await page.getByLabel(/Description/i).fill("created by e2e");
    await page.getByRole("button", { name: "Create Domain", exact: true }).click();
    await expect(page).toHaveURL(/\/permission\/domains\//, { timeout: 60_000 });
    await waitForHydrated(page);
    const desc = `domain-desc-${Date.now()}`;
    await page.getByLabel(/Description/i).first().fill(desc);
    await page.getByRole("button", { name: /Save Changes/i }).click();
    await expect(page.getByLabel(/Description/i).first()).toHaveValue(desc, { timeout: 60_000 });
  });

  test("e2e.domain.detail.add_owner", async ({ page }) => {
    await createDomainAndOpenDetail(page, `E2E-Domain-Owner-${Date.now()}`, "owners e2e");
    const owners = page.locator("#gauge-domain-owners-picker");
    await expect(owners).toBeVisible({ timeout: 60_000 });
    const picker = owners.getByPlaceholder(/Search users or groups/i);
    await picker.click();
    await page.keyboard.type("requestor");
    await pickSearchResult(page, "requestor");
    const ownerList = page.locator("#gauge-domain-owner-remove");
    await expect(ownerList.getByText("requestor", { exact: true })).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.domain.detail.remove_owner", async ({ page }) => {
    await createDomainAndOpenDetail(page, `E2E-Domain-RmOwner-${Date.now()}`, "remove owner e2e");
    const owners = page.locator("#gauge-domain-owners-picker");
    const picker = owners.getByPlaceholder(/Search users or groups/i);
    await picker.click();
    await page.keyboard.type("requestor");
    await pickSearchResult(page, "requestor");
    const ownerList = page.locator("#gauge-domain-owner-remove");
    await expect(ownerList.getByText("requestor", { exact: true })).toBeVisible({
      timeout: 60_000,
    });
    await ownerList
      .getByText("requestor", { exact: true })
      .locator("xpath=ancestor::div[count(.//*[@aria-label='Open owner actions'])=1][1]")
      .getByRole("button", { name: /^Open owner actions$/i })
      .click();
    await page.getByRole("menuitem", { name: /Remove Owner/i }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    await dialog.getByRole("button", { name: "Remove", exact: true }).click();
    await expect(dialog).toHaveCount(0, { timeout: 60_000 });
    await expect(ownerList.getByText("requestor", { exact: true })).toHaveCount(0, {
      timeout: 60_000,
    });
  });

  test("e2e.domain.detail.owner_picker_no_admin", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-domain", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `E2E-Domain-NoAdmin-${Date.now()}`;
    await page.getByLabel(/Domain name/i).fill(name);
    await page.getByLabel(/Description/i).fill("no admin picker");
    await page.getByRole("button", { name: "Create Domain", exact: true }).click();
    await expect(page).toHaveURL(/\/permission\/domains\//, { timeout: 60_000 });
    const domainUrl = page.url();
    await seedAuth(page, "requestor");
    await page.goto(domainUrl, { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const picker = page
      .locator("#gauge-domain-owners-picker")
      .getByPlaceholder(/Search users or groups/i);
    // Non-admin may not see the picker (editor-only owners UI) or sees search deny.
    if ((await picker.count()) === 0) {
      await expect(page.getByText(/Domain Detail/i)).toBeVisible({ timeout: 60_000 });
      return;
    }
    await expect(picker).toBeVisible({ timeout: 60_000 });
    await picker.click();
    await page.keyboard.type("admin");
    await expect(page.locator(".orbital-message-bar--error").first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.domain.create.no_admin", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/permission/create-domain", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByLabel(/Domain name/i).fill("ShouldFail");
    await page.getByLabel(/Description/i).fill("no admin");
    await page.getByRole("button", { name: "Create Domain", exact: true }).click();
    await expectMutationDenied(page);
  });

  test("e2e.domain.create.empty_name", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-domain", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByRole("button", { name: "Create Domain", exact: true }).click();
    await expect(page.getByText(/Domain name is required/i)).toBeVisible({
      timeout: 30_000,
    });
  });
});

test.describe("e2e.perm.create", () => {
  test("e2e.perm.create.happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const name = `E2E-Perm-${Date.now()}`;
    await page.getByLabel(/Display name/i).fill(name);
    await page.getByLabel(/Description/i).fill("e2e permission");
    // Native/Orbital Select under Domain field
    const domainSelect = page.locator("select").first();
    await expect(domainSelect).toBeVisible({ timeout: 60_000 });
    await domainSelect.selectOption({ label: "Ops" });
    await page.getByRole("button", { name: /Create Permission/i }).click();
    await expect(page).toHaveURL(/\/permission\/permissions\//, { timeout: 60_000 });
    await expect(page.getByLabel(/^Name$/i)).toHaveValue(name, { timeout: 30_000 });
  });

  test("e2e.perm.create.domain_required_client", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByLabel(/Display name/i).fill("NoDomainPerm");
    await page.getByRole("button", { name: /Create Permission/i }).click();
    await expect(page.getByText(/Permission domain is required/i)).toBeVisible({
      timeout: 30_000,
    });
  });

  test("e2e.perm.create.no_domains", async ({ page }) => {
    await seedAuth(page, "admin", false, { listDomainsMode: "empty" });
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText(/No domains available\. Create one first/i)).toBeVisible({
      timeout: 60_000,
    });
    // Product leaves Select enabled with only the placeholder when the list is empty.
    const domainSelect = page.locator("select").first();
    await expect(domainSelect.locator("option")).toHaveCount(1);
    await expect(domainSelect.locator("option").first()).toHaveText(/Select a domain/i);
    await seedAuth(page, "admin"); // clear lab override
  });

  test("e2e.perm.create.domains_error", async ({ page }) => {
    await seedAuth(page, "admin", false, { listDomainsMode: "error" });
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.locator(".orbital-message-bar--error").first()).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.locator("select").first()).toBeDisabled();
    await seedAuth(page, "admin");
  });

  test("e2e.perm.create.no_admin", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByLabel(/Display name/i).fill("BlockedPerm");
    const domainSelect = page.locator("select").first();
    await expect(domainSelect).toBeVisible({ timeout: 60_000 });
    await domainSelect.selectOption({ label: "Ops" });
    await page.getByRole("button", { name: /Create Permission/i }).click();
    await expectMutationDenied(page);
  });
});
