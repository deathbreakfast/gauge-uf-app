import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  PEOPLE,
  pickPrincipal,
  stepUpDialog,
  totpCode,
} from "./fixtures";
import type { Page } from "@playwright/test";

async function openGroup(page: Page, id: string) {
  await page.goto(`/permission/groups/${encodeURIComponent(id)}`, { waitUntil: "domcontentloaded" });
  await waitForHydrated(page);
}

async function openTab(page: Page, name: RegExp) {
  await page.locator("#gauge-group-tabs").getByRole("tab", { name }).click();
}

/** Open the row menu for `name` inside `scope` and pick `item`. */
async function rowMenu(page: Page, scope: string, name: string, item: string) {
  await page
    .getByTestId(scope)
    .getByRole("button", { name: new RegExp(`^Actions for ${name}`) })
    .first()
    .click();
  await page.getByRole("menuitem", { name: item, exact: true }).click();
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
    const denied = page.getByTestId("gauge-denied-page");
    await expect(denied.getByText("Creating groups needs GaugeAdmin")).toBeVisible({ timeout: 60_000 });
    await expect(page.locator("#gauge-create-group-form")).toHaveCount(0);
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
    const { fixtures } = await seedAuth(page, "admin", false, { resetReleaseCaptains: true });
    await openGroup(page, fixtures.release_group_id);
    await expect(page.getByRole("heading", { name: "Release Captains" })).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("gauge-group-owner-caption")).toContainText(
      "Owners approve requests to join and add or remove members; each change asks for their authenticator code.",
    );
    const members = page.getByTestId("gauge-group-members");
    await expect(members.getByText(PEOPLE.requestor).first()).toBeVisible({ timeout: 60_000 });
    await expect(members.getByText("On-call SRE").first()).toBeVisible();
    await expect(page.locator("#gauge-group-add-members").getByRole("button")).toBeVisible();
  });

  test("e2e.group.detail.access_tab", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await openGroup(page, fixtures.release_group_id);
    await openTab(page, /Access/);
    const access = page.getByTestId("gauge-group-access");
    await expect(access.getByText("What every member gets from this group.", { exact: false })).toBeVisible();
    await expect(access.getByText("Approve releases", { exact: true })).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.group.detail.remove_shows_loses_keeps_then_code", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", false, {
      resetReleaseCaptains: true,
      stepUpWindow: "none",
    });
    await openGroup(page, fixtures.release_group_id);
    await rowMenu(page, "gauge-group-members", PEOPLE.requestor, "Remove");
    const confirm = page.getByTestId("gauge-remove-confirm");
    await expect(confirm.getByTestId("gauge-remove-loses")).toContainText("Rae loses", { timeout: 30_000 });
    await expect(confirm.getByTestId("gauge-remove-loses")).toContainText("Approve releases");
    await expect(confirm.getByTestId("gauge-remove-keeps")).toContainText("Rae keeps");
    await expect(confirm).toContainText("You'll confirm with your authenticator code.");
    await page.getByTestId("gauge-remove-confirm-button").click();
    const dialog = stepUpDialog(page);
    await expect(dialog.getByTestId("step-up-description")).toContainText(
      `Type the code from your authenticator app to confirm: Remove ${PEOPLE.requestor}`,
      { timeout: 60_000 },
    );
    await dialog.getByTestId("step-up-cancel").getByRole("button").click();
  });

  test("e2e.group.detail.owner_without_admin_removes_member", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider", false, { resetReleaseCaptains: true });
    await openGroup(page, fixtures.release_group_id);
    // Otto owns the group but isn't a Gauge admin: no More menu, but Add members and Remove.
    await expect(page.locator("#gauge-group-add-members").getByRole("button")).toBeVisible({ timeout: 60_000 });
    await expect(page.locator("#gauge-group-more")).toHaveCount(0);
    await rowMenu(page, "gauge-group-members", PEOPLE.requestor, "Remove");
    await page.getByTestId("gauge-remove-confirm-button").click();
    const notice = page.getByTestId("gauge-group-notice");
    await expect(notice).toContainText(`${PEOPLE.requestor} removed from Release Captains`, { timeout: 60_000 });
    await expect(notice.getByTestId("gauge-group-undo")).toBeVisible();
    await notice.getByTestId("gauge-group-undo").getByRole("button").click();
    await expect(page.getByTestId("gauge-group-members").getByText(PEOPLE.requestor).first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.group.detail.owner_removal_needs_code_without_window", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", false, {
      resetReleaseCaptains: true,
      stepUpWindow: "none",
    });
    await openGroup(page, fixtures.release_group_id);
    await openTab(page, /Owners/);
    await rowMenu(page, "gauge-group-owners", PEOPLE.outsider, "Remove as owner");
    const confirm = page.getByTestId("gauge-group-owner-remove-confirm");
    await expect(confirm).toContainText("Otto stops approving requests to join Release Captains");
    await page.getByTestId("gauge-group-owner-remove-button").click();
    await expect(stepUpDialog(page).getByTestId("step-up-description")).toContainText(
      `to confirm: Remove ${PEOPLE.outsider} as an owner of Release Captains.`,
      { timeout: 60_000 },
    );
    await stepUpDialog(page).getByTestId("step-up-cancel").getByRole("button").click();
  });

  test("e2e.group.detail.owner_removal_with_recent_code", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", false, { resetReleaseCaptains: true });
    await openGroup(page, fixtures.release_group_id);
    await openTab(page, /Owners/);
    await rowMenu(page, "gauge-group-owners", PEOPLE.outsider, "Remove as owner");
    await page.getByTestId("gauge-group-owner-remove-button").click();
    await expect(page.getByTestId("gauge-group-notice")).toContainText(
      `${PEOPLE.outsider} is no longer an owner of Release Captains`,
      { timeout: 60_000 },
    );
    await expect(stepUpDialog(page)).toHaveCount(0);
    await seedAuth(page, "admin", false, { resetReleaseCaptains: true });
  });

  test("e2e.group.detail.super_user_owner_removal_asks_fresh_code", async ({ page }) => {
    // Kenji owns Super User; only its owners and Super Users can see who owns it.
    await seedAuth(page, "kenji");
    await openGroup(page, "super_user_group");
    await expect(page.getByTestId("gauge-group-super-user-warning")).toContainText(
      "Members can do everything in every app",
      { timeout: 60_000 },
    );
    await openTab(page, /Owners/);
    await rowMenu(page, "gauge-group-owners", PEOPLE.kenji, "Remove as owner");
    const confirm = page.getByTestId("gauge-group-owner-remove-confirm");
    await expect(confirm).toContainText("Super User changes need a fresh authenticator code.");
    await page.getByTestId("gauge-group-owner-remove-button").click();
    const dialog = stepUpDialog(page);
    // A recent code is on file, but Super User still asks for a fresh one.
    await expect(dialog.getByTestId("step-up-description")).toContainText(
      `to confirm: Remove ${PEOPLE.kenji} as an owner of Super User.`,
      { timeout: 60_000 },
    );
    await expect(dialog.getByTestId("step-up-description")).toContainText(
      "Super User changes need a fresh code, even if you typed one a minute ago.",
    );
    await dialog.getByTestId("step-up-cancel").getByRole("button").click();
  });

  test("e2e.group.detail.add_member_with_code", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", false, { stepUpWindow: "none" });
    await openGroup(page, fixtures.child_group_id);
    await page.locator("#gauge-group-add-members").getByRole("button").click();
    await pickPrincipal(page, "gauge-group-members-picker", "Otto", PEOPLE.outsider);
    const dialog = stepUpDialog(page);
    await expect(dialog).toBeVisible({ timeout: 60_000 });
    await dialog.getByTestId("step-up-totp").locator("input").fill(totpCode());
    await dialog.getByTestId("step-up-submit").getByRole("button").click();
    await expect(page.getByTestId("gauge-group-members").getByText(PEOPLE.outsider).first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.group.detail.activity_names_people", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", false, { resetReleaseCaptains: true });
    await openGroup(page, fixtures.release_group_id);
    await openTab(page, /Activity/);
    const activity = page.getByTestId("gauge-record-activity");
    await expect(activity.getByText(new RegExp(`Added ${PEOPLE.requestor}.* to the group`)).first()).toBeVisible({
      timeout: 60_000,
    });
    await expect(activity.getByText(PEOPLE.admin, { exact: false }).first()).toBeVisible();
    await expect(activity.getByText(/user:/)).toHaveCount(0);
  });

  test("e2e.group.detail.not_found", async ({ page }) => {
    await seedAuth(page, "admin");
    await openGroup(page, "does-not-exist-xyz");
    await expect(page.getByTestId("gauge-group-not-found").getByText("No group with that link")).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.group.detail.request_access_for_non_owner", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await openGroup(page, fixtures.group_id);
    await expect(page.locator("#gauge-group-request-access").getByRole("button", { name: "Request access" })).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.locator("#gauge-group-add-members").getByRole("button")).toHaveCount(0);
  });
});
