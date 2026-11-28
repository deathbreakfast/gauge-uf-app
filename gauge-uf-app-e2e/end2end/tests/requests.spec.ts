import type { Page } from "@playwright/test";
import { test, expect, seedAuth, waitForHydrated, PEOPLE } from "./fixtures";

/** Deny the open request with `reason` through the reason dialog. */
async function denyWithReason(page: Page, reason: string) {
  await page.getByRole("button", { name: "Deny", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog.getByTestId("gauge-request-deny-dialog")).toBeVisible({ timeout: 30_000 });
  await dialog.locator("textarea").fill(reason);
  await dialog.getByRole("button", { name: "Deny request", exact: true }).click();
  await expect(page.getByTestId("gauge-request-status")).toHaveText(/Denied/, { timeout: 60_000 });
}

test.describe("e2e.req.index", () => {
  test("e2e.req.index.reviewer_sees_pending", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-requests-index")).toBeVisible({ timeout: 60_000 });
    await expect(
      page.locator(`a[href="/permission/requests/${fixtures.pending_perm_request_id}"]`),
    ).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.req.index.requestor_sees_own", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor", true);
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-requests-index")).toBeVisible({ timeout: 60_000 });
    await page.locator("#gauge-requests-tabs").getByRole("tab", { name: "Your requests" }).click();
    await expect(page).toHaveURL(/tab=mine/);
    await expect(
      page.locator(`a[href="/permission/requests/${fixtures.pending_perm_request_id}"]`),
    ).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.req.index.review_list_permission", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const waiting = page.locator("#gauge-requests-needs-review");
    await expect(
      waiting.locator(`a[href="/permission/requests/${fixtures.pending_perm_request_id}"]`),
    ).toBeVisible({ timeout: 60_000 });
    await expect(waiting.getByText("Permission", { exact: true }).first()).toBeVisible();
    await expect(waiting.getByTestId("gauge-request-status").first()).toHaveText("Waiting");
  });

  test("e2e.req.index.review_list_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const waiting = page.locator("#gauge-requests-needs-review");
    await expect(
      waiting.locator(`a[href="/permission/requests/${fixtures.pending_group_request_id}"]`),
    ).toBeVisible({ timeout: 60_000 });
    await expect(waiting.getByText("Group", { exact: true }).first()).toBeVisible();
  });
});

test.describe("e2e.req.detail", () => {
  // Deny before approve so refresh-minted requestor pending ids stay PENDING.
  test("e2e.req.detail.deny_permission", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto(`/permission/requests/${fixtures.pending_perm_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await denyWithReason(page, "Not this quarter.");
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByRole("button", { name: /Open member actions/i })).toHaveCount(0);
  });

  test("e2e.req.detail.deny_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto(`/permission/requests/${fixtures.pending_group_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await denyWithReason(page, "Not this quarter.");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-group-members")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("gauge-group-members").getByText(PEOPLE.requestor)).toHaveCount(0);
  });

  test("e2e.req.detail.approve_permission", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto(`/permission/requests/${fixtures.pending_perm_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-request-detail")).toBeVisible({ timeout: 60_000 });
    await page.getByRole("button", { name: "Approve", exact: true }).click();
    await expect(page.getByTestId("gauge-request-status")).toHaveText(/Approved/, { timeout: 60_000 });
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await page.locator("#gauge-perm-tabs").getByRole("tab", { name: /Allow list/ }).click();
    await expect(page.getByTestId("gauge-perm-allow").getByText(PEOPLE.requestor).first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.req.detail.approve_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto(`/permission/requests/${fixtures.pending_group_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await page.getByRole("button", { name: "Approve", exact: true }).click();
    await expect(page.getByTestId("gauge-request-status")).toHaveText(/Approved/, { timeout: 60_000 });
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-group-members").getByText(PEOPLE.requestor).first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.req.detail.terminal_denied", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto(`/permission/requests/${fixtures.pending_perm_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await denyWithReason(page, "Not this quarter.");
    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-request-status")).toHaveText(/Denied/, { timeout: 60_000 });
    await expect(page.getByRole("button", { name: "Approve", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Deny", exact: true })).toHaveCount(0);
  });

  test("e2e.req.detail.terminal_approved", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto(`/permission/requests/${fixtures.pending_group_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await page.getByRole("button", { name: "Approve", exact: true }).click();
    await expect(page.getByTestId("gauge-request-status")).toHaveText(/Approved/, { timeout: 60_000 });
    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-request-status")).toHaveText(/Approved/, { timeout: 60_000 });
    await expect(page.getByRole("button", { name: "Approve", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Deny", exact: true })).toHaveCount(0);
  });

  test("e2e.req.detail.no_actions_requestor", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor", true);
    await page.goto(`/permission/requests/${fixtures.pending_perm_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-request-detail")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByRole("button", { name: "Approve", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Deny", exact: true })).toHaveCount(0);
  });

  test("e2e.req.detail.no_actions_outsider", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await seedAuth(page, "outsider", false);
    await page.goto(`/permission/requests/${fixtures.pending_perm_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByRole("button", { name: "Approve", exact: true })).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Deny", exact: true })).toHaveCount(0);
  });

  test("e2e.req.detail.unauthorized_viewer", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await seedAuth(page, "outsider", false);
    await page.goto(`/permission/requests/${fixtures.pending_perm_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(
      page.getByTestId("gauge-error-bar").getByText("You don't have access"),
    ).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.req.detail.not_found", async ({ page }) => {
    await seedAuth(page, "admin", false);
    await page.goto("/permission/requests/missing-request-id", {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByText(/Request not found/i)).toBeVisible({ timeout: 60_000 });
  });
});

test.describe("e2e.req.decide", () => {
  test("e2e.req.decide.deny_needs_reason_and_requester_sees_it", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto(`/permission/requests/${fixtures.pending_group_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await page.getByRole("button", { name: "Deny", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: "Deny request", exact: true }).click();
    await expect(dialog.getByText(/Say why/)).toBeVisible({ timeout: 30_000 });
    const reason = `Kenji covers November ${Date.now()}`;
    await dialog.locator("textarea").fill(reason);
    await dialog.getByRole("button", { name: "Deny request", exact: true }).click();
    const decision = page.getByTestId("gauge-request-decision");
    await expect(decision.getByText(/^Denied by /)).toBeVisible({ timeout: 60_000 });

    await seedAuth(page, "requestor", false);
    await page.goto(`/permission/requests/${fixtures.pending_group_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-request-decision").getByText(reason, { exact: false })).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.req.decide.withdraw_leaves_reviewer_queue", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor", true);
    const id = fixtures.pending_perm_request_id;
    await page.goto(`/permission/requests/${id}`, { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-request-who-reviews").getByText(/reviews this$/)).toBeVisible({
      timeout: 60_000,
    });
    await page.locator("#gauge-request-withdraw").getByRole("button").click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText(/^Withdraw your request/)).toBeVisible({ timeout: 30_000 });
    await dialog.locator("#gauge-request-withdraw-confirm").getByRole("button").click();
    await expect(page.getByTestId("gauge-request-status")).toHaveText(/Withdrawn/, { timeout: 60_000 });

    await seedAuth(page, "admin", false);
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.locator("#gauge-requests-needs-review")).toBeVisible({ timeout: 60_000 });
    await expect(page.locator(`a[href="/permission/requests/${id}"]`)).toHaveCount(0);
  });

  test("e2e.req.access.names_reviewer_and_confirms", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "outsider");
    await page.goto(`/permission/groups/${fixtures.group_id}?request=1`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText(/^Ask to join /)).toBeVisible({ timeout: 60_000 });
    const facts = dialog.getByTestId("gauge-request-facts");
    await expect(facts.getByText("Reviewed by")).toBeVisible();
    await expect(facts.getByText("Members get")).toBeVisible();
    await dialog.locator("textarea").fill(`outsider asks ${Date.now()}`);
    await dialog.getByRole("button", { name: "Send request", exact: true }).click();
    await expect(dialog.getByText(/^Request sent to /)).toBeVisible({ timeout: 60_000 });
    await dialog.getByRole("button", { name: "Done", exact: true }).click();

    await page.goto(`/permission/groups/${fixtures.group_id}?request=1`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByRole("dialog").getByText("You already asked")).toBeVisible({
      timeout: 60_000,
    });
  });
});
