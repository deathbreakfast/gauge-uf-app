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

async function openPermission(page: Page, id: string) {
  await page.goto(`/permission/permissions/${encodeURIComponent(id)}`, {
    waitUntil: "domcontentloaded",
  });
  await waitForHydrated(page);
}

async function openTab(page: Page, name: RegExp) {
  await page.locator("#gauge-perm-tabs").getByRole("tab", { name }).click();
}

/** Add `who` to the allow list through Add to allow list (a recent code is on file). */
async function addToAllowList(page: Page, query: string, name: string) {
  await openTab(page, /Allow list/);
  const list = page.getByTestId("gauge-perm-allow");
  if ((await list.getByText(name).count()) > 0) return;
  await page.locator("#gauge-perm-allow-add").getByRole("button").click();
  const dialog = page.getByTestId("gauge-give-access");
  await expect(dialog).toBeVisible({ timeout: 15_000 });
  await pickPrincipal(page, "gauge-give-access-picker", query, name);
  await page.getByTestId("gauge-give-access-submit").click();
  await expect(page.getByTestId("gauge-permission-notice")).toBeVisible({ timeout: 60_000 });
  await openTab(page, /Allow list/);
  await expect(list.getByText(name).first()).toBeVisible({ timeout: 60_000 });
}

test.describe("e2e.perm.detail", () => {
  test("e2e.perm.detail.load_happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await openPermission(page, fixtures.release_permission_id);
    await expect(page.getByRole("heading", { name: "Approve releases" })).toBeVisible({
      timeout: 60_000,
    });
    // The description renders as Markdown, and the key shows as code.
    await expect(page.getByTestId("gauge-perm-description").locator("strong")).toHaveText("before");
    await expect(page.locator("#gauge-perm-detail-header").getByText("ApproveRelease", { exact: true })).toBeVisible();
    await expect(page.getByTestId("gauge-perm-facts").getByText("Owners of Approve releases")).toBeVisible();
    await expect(page.locator("#gauge-perm-give").getByRole("button", { name: "Give access" })).toBeVisible();
  });

  test("e2e.perm.detail.who_has_access_nested_path", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await openPermission(page, fixtures.release_permission_id);
    const who = page.getByTestId("gauge-perm-who");
    await expect(who.getByText(PEOPLE.kenji).first()).toBeVisible({ timeout: 60_000 });
    await expect(who.getByText("Through Release Captains › On-call SRE").first()).toBeVisible();
    await expect(who.getByText(PEOPLE.requestor).first()).toBeVisible();
  });

  test("e2e.perm.detail.give_access_suggests_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await openPermission(page, fixtures.release_permission_id);
    await page.locator("#gauge-perm-give").getByRole("button", { name: "Give access" }).click();
    const dialog = page.getByTestId("gauge-give-access");
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    await pickPrincipal(page, "gauge-give-access-picker", "Otto", PEOPLE.outsider);
    const suggest = page.getByTestId("gauge-give-access-suggest");
    await expect(suggest.getByText("Release Captains already has this permission")).toBeVisible({
      timeout: 30_000,
    });
    await expect(dialog.getByText("Add Otto to Release Captains (suggested)")).toBeVisible();
    await expect(dialog.getByText("Put Otto on the allow list")).toBeVisible();
    await expect(page.getByTestId("gauge-give-access-submit")).toHaveText("Add to Release Captains");
    await page.getByRole("button", { name: "Cancel" }).last().click();
  });

  test("e2e.perm.detail.grant_and_remove_user", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await openPermission(page, fixtures.permission_id);
    await addToAllowList(page, "Rae", PEOPLE.requestor);
    const list = page.getByTestId("gauge-perm-allow");
    await expect(list.getByText("Removing one takes away only what comes from this list.", { exact: false })).toBeVisible();
    await list.getByRole("button", { name: new RegExp(`^Actions for ${PEOPLE.requestor}`) }).click();
    await page.getByRole("menuitem", { name: "Remove", exact: true }).click();
    const confirm = page.getByTestId("gauge-remove-confirm");
    await expect(confirm.getByTestId("gauge-remove-loses")).toContainText("Rae loses", { timeout: 30_000 });
    await expect(confirm.getByTestId("gauge-remove-loses")).toContainText("CanDeploy");
    await expect(confirm.getByTestId("gauge-remove-keeps")).toContainText("Rae keeps");
    await page.getByTestId("gauge-remove-confirm-button").click();
    await expect(page.getByTestId("gauge-permission-notice")).toContainText("is off the allow list", {
      timeout: 60_000,
    });
  });

  test("e2e.perm.detail.non_admin_sees_owner_line", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await openPermission(page, fixtures.permission_id);
    await expect(page.getByTestId("gauge-perm-why-not").getByText("You don't have this permission")).toBeVisible({
      timeout: 60_000,
    });
    await expect(page.getByTestId("gauge-perm-owner-line")).toContainText(
      "Only the owners of this permission and Gauge admins can see who holds it.",
    );
    await expect(page.locator("#gauge-perm-request-access").getByRole("button", { name: "Request access" })).toBeVisible();
    await expect(page.locator("#gauge-perm-tabs")).toHaveCount(0);
    await expect(page.locator("#gauge-perm-give").getByRole("button")).toHaveCount(0);
  });

  test("e2e.perm.detail.request_submit_happy", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await openPermission(page, fixtures.permission_id);
    const requestBtn = page.locator("#gauge-perm-request-access").getByRole("button");
    await expect(requestBtn).toBeVisible({ timeout: 60_000 });
    await requestBtn.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible({ timeout: 15_000 });
    await dialog.locator("textarea").fill(`outsider needs access ${Date.now()}`);
    await dialog.getByRole("button", { name: "Submit Request", exact: true }).click();
    await expect(dialog).toHaveCount(0, { timeout: 60_000 });
  });

  test("e2e.perm.detail.request_empty_reason", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await openPermission(page, fixtures.permission_id);
    const requestBtn = page.locator("#gauge-perm-request-access").getByRole("button");
    await expect(requestBtn).toBeVisible({ timeout: 60_000 });
    await requestBtn.click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: "Submit Request", exact: true }).click();
    await expect(page.getByText(/Reason is required/i)).toBeVisible({ timeout: 30_000 });
  });

  test("e2e.perm.detail.not_found", async ({ page }) => {
    await seedAuth(page, "admin");
    await openPermission(page, "does-not-exist-xyz");
    const missing = page.getByTestId("gauge-permission-not-found");
    await expect(missing.getByText("No permission with that link")).toBeVisible({ timeout: 60_000 });
    await missing.getByRole("button", { name: "Back to Permissions" }).click();
    await expect(page).toHaveURL(/\/permission\/permissions$/);
  });

  test("e2e.perm.detail.delete_needs_key", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await openPermission(page, fixtures.release_permission_id);
    await page.locator("#gauge-perm-more").getByRole("button", { name: "More actions" }).click();
    await page.getByRole("menuitem", { name: "Delete" }).click();
    const confirm = page.getByTestId("gauge-perm-delete-confirm");
    await expect(confirm.getByText(/lose Approve releases, including everyone in Release Captains\./)).toBeVisible({
      timeout: 30_000,
    });
    await expect(confirm.getByText(/Code that checks ApproveRelease will refuse everyone except Super User/)).toBeVisible();
    const del = page.locator("#gauge-perm-delete").getByRole("button");
    await expect(del).toBeDisabled();
    await confirm.locator("input").fill("ApproveReleas");
    await expect(del).toBeDisabled();
    await confirm.locator("input").fill("ApproveRelease");
    await expect(del).toBeEnabled();
    await page.getByRole("button", { name: "Cancel" }).last().click();
  });

  test("e2e.perm.detail.activity_names_people", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await openPermission(page, fixtures.release_permission_id);
    await openTab(page, /Activity/);
    const activity = page.getByTestId("gauge-record-activity");
    await expect(activity.getByText(/Gave the group Release Captains Approve releases/).first()).toBeVisible({
      timeout: 60_000,
    });
    await expect(activity.getByText(PEOPLE.admin, { exact: false }).first()).toBeVisible();
    // Times go through Orbital Timestamp, never raw RFC 3339 text.
    await expect(activity.locator("time[datetime]").first()).toBeVisible();
    await expect(activity.getByText(/\d{4}-\d{2}-\d{2}T\d{2}:/)).toHaveCount(0);
  });

  test("e2e.perm.detail.wrong_code_sentence", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", false, { stepUpWindow: "none" });
    await openPermission(page, fixtures.release_permission_id);
    await page.locator("#gauge-perm-give").getByRole("button", { name: "Give access" }).click();
    await pickPrincipal(page, "gauge-give-access-picker", "Otto", PEOPLE.outsider);
    await page.getByTestId("gauge-give-access-submit").click();
    const dialog = stepUpDialog(page);
    await expect(dialog.getByTestId("step-up-description")).toContainText(
      "Type the code from your authenticator app to confirm: Add Otto Outsider to Release Captains.",
      { timeout: 60_000 },
    );
    const wrong = totpCode() === "000000" ? "111111" : "000000";
    await dialog.getByTestId("step-up-totp").locator("input").fill(wrong);
    await dialog.getByTestId("step-up-submit").getByRole("button").click();
    await expect(dialog.getByTestId("step-up-error")).toContainText("That code didn't work.", {
      timeout: 30_000,
    });
    await expect(dialog.getByTestId("step-up-error")).toContainText("Codes change every 30 seconds");
    await expect(dialog.getByTestId("step-up-error")).not.toContainText("STEP_UP");
    await dialog.getByTestId("step-up-cancel").getByRole("button").click();
  });
});
