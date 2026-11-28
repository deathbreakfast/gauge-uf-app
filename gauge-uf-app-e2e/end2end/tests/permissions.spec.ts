import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  expectMutationDenied,
  PEOPLE,
  pickPrincipal,
  userRow,
} from "./fixtures";

test.describe("e2e.perm.index", () => {
  test("e2e.perm.index.load_happy", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-permissions-index")).toBeVisible({
      timeout: 60_000,
    });
    const table = page.getByTestId("gauge-permissions-table");
    await expect(table.getByText("Approve releases", { exact: true })).toBeVisible({ timeout: 60_000 });
    // The key shows as code under the name, and admins see who holds each one.
    await expect(table.getByText("ApproveRelease", { exact: true })).toBeVisible();
    await expect(table.getByText(/\d+ (person|people)/).first()).toBeVisible();
  });

  test("e2e.perm.index.search_match", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const table = page.getByTestId("gauge-permissions-table");
    await expect(table.getByText("CanDeploy", { exact: true }).first()).toBeVisible({ timeout: 60_000 });
    await table.getByRole("searchbox").or(table.getByRole("textbox")).first().fill("ApproveRelease");
    await expect(table.getByText("Approve releases", { exact: true })).toBeVisible();
    await expect(page).toHaveURL(/[?&]q=ApproveRelease/);
  });

  test("e2e.perm.index.search_no_match", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/permissions?q=zz-no-such-perm", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const noMatch = page.getByTestId("gauge-permissions-no-match");
    await expect(noMatch).toBeVisible({ timeout: 60_000 });
    await expect(noMatch.getByText("No permission matches your search")).toBeVisible();
    await expect(page.getByTestId("gauge-permissions-empty")).toHaveCount(0);
    await noMatch.getByRole("button", { name: "Clear search" }).click();
    await expect(page.getByText("CanDeploy", { exact: true }).first()).toBeVisible({ timeout: 30_000 });
  });

  test("e2e.perm.index.open_row", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.getByTestId("gauge-permissions-table").getByText("Approve releases", { exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/permission/permissions/${fixtures.release_permission_id}`));
  });

  test("e2e.perm.index.create_cta", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.locator("#gauge-permissions-create").getByRole("button", { name: "Create permission" }).click();
    await expect(page).toHaveURL(/\/permission\/create-permission/);
  });

  test("e2e.perm.index.no_create_for_non_admin", async ({ page }) => {
    await seedAuth(page, "outsider");
    await page.goto("/permission/permissions", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText("CanDeploy", { exact: true }).first()).toBeVisible({ timeout: 60_000 });
    await expect(page.locator("#gauge-permissions-create")).toHaveCount(0);
  });

});

test.describe("e2e.domain.create", () => {
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
    await pickPrincipal(page, "gauge-domain-owners-picker", "Rae", PEOPLE.requestor);
    const ownerList = page.locator("#gauge-domain-owner-remove");
    await expect(ownerList.getByText(userRow("requestor"))).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.domain.detail.remove_owner", async ({ page }) => {
    await createDomainAndOpenDetail(page, `E2E-Domain-RmOwner-${Date.now()}`, "remove owner e2e");
    const owners = page.locator("#gauge-domain-owners-picker");
    await pickPrincipal(page, "gauge-domain-owners-picker", "Rae", PEOPLE.requestor);
    const ownerList = page.locator("#gauge-domain-owner-remove");
    await expect(ownerList.getByText(userRow("requestor"))).toBeVisible({
      timeout: 60_000,
    });
    await ownerList
      .getByText(userRow("requestor"))
      .locator("xpath=ancestor::div[count(.//*[@aria-label='Open owner actions'])=1][1]")
      .getByRole("button", { name: /^Open owner actions$/i })
      .click();
    await page.getByRole("menuitem", { name: /Remove Owner/i }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    await dialog.getByRole("button", { name: "Remove", exact: true }).click();
    await expect(dialog).toHaveCount(0, { timeout: 60_000 });
    await expect(ownerList.getByText(userRow("requestor"))).toHaveCount(0, {
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
    const picker = page.locator("#gauge-domain-owners-picker").getByRole("combobox");
    // Non-admin may not see the picker (editor-only owners UI) or gets the add refused.
    if ((await picker.count()) === 0) {
      await expect(page.getByText(/Domain Detail/i)).toBeVisible({ timeout: 60_000 });
      return;
    }
    await pickPrincipal(page, "gauge-domain-owners-picker", "Otto", PEOPLE.outsider);
    await expectMutationDenied(page);
    await expect(
      page.locator("#gauge-domain-owner-remove").getByText(userRow("outsider")),
    ).toHaveCount(0);
  });

  test("e2e.domain.create.no_admin", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/permission/create-domain", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const denied = page.getByTestId("gauge-denied-page");
    await expect(denied.getByText("Creating domains needs GaugeAdmin")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByRole("button", { name: "Create Domain", exact: true })).toHaveCount(0);
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
    const stamp = Date.now();
    const name = `E2E perm ${stamp}`;
    await page.locator("#gauge-create-perm-name input").fill(name);
    // The key follows the name.
    await expect(page.locator("#gauge-create-perm-key input")).toHaveValue(`E2EPerm${stamp}`);
    await expect(page.getByTestId("gauge-create-perm-owner-default")).toContainText(
      `Owners of ${name} (new group, you are its owner)`,
    );
    await page.getByTestId("gauge-create-perm-description").locator("textarea").fill("Lets **e2e** run.");
    const domainSelect = page.locator("select").first();
    await expect(domainSelect).toBeVisible({ timeout: 60_000 });
    await domainSelect.selectOption({ label: "Ops" });
    await page.getByRole("button", { name: "Create permission" }).click();
    await expect(page).toHaveURL(/\/permission\/permissions\//, { timeout: 60_000 });
    await expect(page.getByRole("heading", { name })).toBeVisible({ timeout: 30_000 });
    await expect(page.getByText(`E2EPerm${stamp}`, { exact: true })).toBeVisible();
  });

  test("e2e.perm.create.domain_required_client", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await page.locator("#gauge-create-perm-name input").fill("NoDomainPerm");
    await page.getByRole("button", { name: "Create permission" }).click();
    await expect(page.getByTestId("gauge-error-bar").getByText("Pick a domain.")).toBeVisible({
      timeout: 30_000,
    });
  });

  test("e2e.perm.create.no_domains", async ({ page }) => {
    await seedAuth(page, "admin", false, { listDomainsMode: "empty" });
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText(/No domains yet\. Create one first/i)).toBeVisible({
      timeout: 60_000,
    });
    const domainSelect = page.locator("select").first();
    await expect(domainSelect.locator("option")).toHaveCount(1);
    await seedAuth(page, "admin"); // clear lab override
  });

  test("e2e.perm.create.domains_error", async ({ page }) => {
    await seedAuth(page, "admin", false, { listDomainsMode: "error" });
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const bar = page.getByTestId("gauge-error-bar");
    await expect(bar.getByText("Couldn't load the domains")).toBeVisible({ timeout: 60_000 });
    // Never the raw server error.
    await expect(page.getByText(/e2e forced failure/)).toHaveCount(0);
    await expect(page.locator("select").first()).toBeDisabled();
    await seedAuth(page, "admin");
  });

  test("e2e.perm.create.no_admin", async ({ page }) => {
    await seedAuth(page, "requestor");
    await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const denied = page.getByTestId("gauge-denied-page");
    await expect(denied.getByText("Creating permissions needs GaugeAdmin")).toBeVisible({ timeout: 60_000 });
    await expect(denied.getByRole("button", { name: "Ask for GaugeAdmin" })).toBeVisible();
    await expect(page.locator("#gauge-create-perm-form")).toHaveCount(0);
  });

});
