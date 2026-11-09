import { test, expect, seedAuth, waitForHydrated, userRow } from "./fixtures";

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
    await expect(page.getByText("My Requests", { exact: true })).toBeVisible({ timeout: 60_000 });
    await expect(
      page.locator(`a[href="/permission/requests/${fixtures.pending_perm_request_id}"]`),
    ).toBeVisible({ timeout: 60_000 });
  });

  test("e2e.req.index.review_list_permission", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText("Needs Review", { exact: true })).toBeVisible({ timeout: 60_000 });
    await expect(
      page.locator(`a[href="/permission/requests/${fixtures.pending_perm_request_id}"]`),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText(/Permission - PENDING/i).first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.req.index.review_list_group", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText("Needs Review", { exact: true })).toBeVisible({ timeout: 60_000 });
    await expect(
      page.locator(`a[href="/permission/requests/${fixtures.pending_group_request_id}"]`),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText(/Group - PENDING/i).first()).toBeVisible({
      timeout: 60_000,
    });
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
    await page.getByRole("button", { name: "Deny", exact: true }).click();
    await expect(page.getByText(/DENIED/i)).toBeVisible({ timeout: 60_000 });
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
    await page.getByRole("button", { name: "Deny", exact: true }).click();
    await expect(page.getByText(/DENIED/i)).toBeVisible({ timeout: 60_000 });
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const memberActions = page.getByRole("button", { name: /Open member actions/i });
    const count = await memberActions.count();
    for (let i = 0; i < count; i++) {
      const text = await memberActions
        .nth(i)
        .locator("xpath=ancestor::div[1]")
        .innerText()
        .catch(() => "");
      expect(text.includes("requestor")).toBeFalsy();
    }
  });

  test("e2e.req.detail.approve_permission", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto(`/permission/requests/${fixtures.pending_perm_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-request-detail")).toBeVisible({ timeout: 60_000 });
    await page.getByRole("button", { name: "Approve", exact: true }).click();
    await expect(page.getByText(/APPROVED/i)).toBeVisible({ timeout: 60_000 });
    await page.goto(`/permission/permissions/${fixtures.permission_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByText(userRow("requestor")).first()).toBeVisible({
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
    await expect(page.getByText(/APPROVED/i)).toBeVisible({ timeout: 60_000 });
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByText(userRow("requestor")).first()).toBeVisible({
      timeout: 60_000,
    });
  });

  test("e2e.req.detail.terminal_denied", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin", true);
    await page.goto(`/permission/requests/${fixtures.pending_perm_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await page.getByRole("button", { name: "Deny", exact: true }).click();
    await expect(page.getByText(/DENIED/i)).toBeVisible({ timeout: 60_000 });
    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText(/DENIED/i)).toBeVisible({ timeout: 60_000 });
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
    await expect(page.getByText(/APPROVED/i)).toBeVisible({ timeout: 60_000 });
    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText(/APPROVED/i)).toBeVisible({ timeout: 60_000 });
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
      page.getByText(/Not authorized to view this request/i).first(),
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
