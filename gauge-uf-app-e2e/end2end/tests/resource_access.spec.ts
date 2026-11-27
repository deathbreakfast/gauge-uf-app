import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

// The host seeds a Chronon job bundle owned by "Chronon admins" (admin owns that group),
// an active team group "Billing engineering" with Can edit, an archived team group
// "Payments on-call" with Can run, and a pending Can view request from requestor.
const JOB = "Chronon job billing-rollup";

test.describe("e2e.rag.groups", () => {
  test("e2e.rag.groups.teams_tab", async ({ page }) => {
    await seedAuth(page, "admin");
    await page.goto("/permission/groups", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    const list = page.locator("#gauge-groups-list");
    await expect(list.getByText("Deployers", { exact: true })).toBeVisible({ timeout: 60_000 });

    await page.getByTestId("gauge-groups-tabs").getByRole("tab", { name: "Teams" }).click();
    await expect(page.getByTestId("gauge-groups-teams-caption")).toBeVisible();
    await expect(list.getByText("Deployers", { exact: true })).toHaveCount(0);
    await expect(list.getByText("Billing engineering", { exact: true })).toBeVisible();
    await expect(list.getByText("Payments on-call", { exact: true })).toBeVisible();
    await expect(list.getByTestId("gauge-group-team-badge")).toHaveCount(2);
    await expect(list.getByTestId("gauge-group-archived-badge")).toHaveCount(1);
    await expect(list.getByText(/Members come from Polaron · 2 members/)).toBeVisible();
    await expect(list.getByText(/archived, no members/)).toBeVisible();

    await page.getByTestId("gauge-groups-tabs").getByRole("tab", { name: "All groups" }).click();
    await expect(list.getByText("Deployers", { exact: true })).toBeVisible();
  });
});

test.describe("e2e.rag.group", () => {
  test("e2e.rag.group.team_page", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.team_group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const banner = page.getByTestId("gauge-group-managed-banner");
    await expect(banner).toContainText(
      "Members come from the Billing engineering team in Polaron",
      { timeout: 60_000 },
    );
    await expect(banner.getByRole("link", { name: "Open in Polaron" })).toHaveAttribute(
      "href",
      "/polaron/teams/t81kq2",
    );
    await expect(page.getByTestId("gauge-group-team-badge")).toBeVisible();

    const members = page.getByTestId("gauge-group-managed-members");
    await expect(members.getByText("Members (2)")).toBeVisible();
    await expect(members.getByText("Read-only: Polaron manages this list.")).toBeVisible();
    // No way to edit the roster here.
    await expect(page.locator("#gauge-group-members-picker")).toHaveCount(0);
    await expect(page.getByRole("button", { name: /Open member actions/i })).toHaveCount(0);

    const access = page.getByTestId("gauge-group-resource-access");
    await expect(access.getByText(JOB)).toBeVisible({ timeout: 60_000 });
    await expect(access.getByText("Can edit", { exact: true })).toBeVisible();
  });

  test("e2e.rag.group.archived_team", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.archived_team_group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-group-managed-banner")).toContainText(
      "Payments on-call is archived in Polaron",
      { timeout: 60_000 },
    );
    await expect(page.getByTestId("gauge-group-archived-badge")).toBeVisible();
    await expect(
      page.getByTestId("gauge-group-managed-members").getByText(
        "No members while the team is archived.",
      ),
    ).toBeVisible();
    const access = page.getByTestId("gauge-group-resource-access");
    await expect(access.getByText(JOB)).toBeVisible({ timeout: 60_000 });
    await expect(access.getByText("Can run", { exact: true })).toBeVisible();
  });

  test("e2e.rag.group.plain_group_has_no_access", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/groups/${fixtures.group_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(
      page.getByTestId("gauge-group-resource-access").getByText(
        "This group has no role on any resource.",
      ),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("gauge-group-managed-banner")).toHaveCount(0);
    await expect(page.locator("#gauge-group-members-picker")).toBeVisible();
  });
});

test.describe("e2e.rag.domain", () => {
  test("e2e.rag.domain.resource_card", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/domains/${fixtures.resource_domain_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const card = page.getByTestId("gauge-resource-bundle-card");
    await expect(card.getByTestId("gauge-resource-title")).toHaveText(JOB, { timeout: 60_000 });
    await expect(card.getByRole("link", { name: "Open in Chronon" })).toHaveAttribute(
      "href",
      "/chronon/jobs/billing-rollup",
    );
    await expect(card.getByText("The default owner is Chronon admins.")).toBeVisible();

    const permissions = card.getByTestId("gauge-resource-permissions");
    const permissionRow = (action: string) =>
      permissions
        .getByRole("row")
        .filter({ has: page.getByRole("link", { name: new RegExp(`\\.${action}$`) }) });
    const viewRow = permissionRow("View");
    await expect(viewRow).toContainText("Owners");
    await expect(viewRow).toContainText("Billing engineering");
    await expect(viewRow).toContainText("Payments on-call");
    const maintainRow = permissionRow("Maintain");
    await expect(maintainRow).not.toContainText("Billing engineering");

    const roles = card.getByTestId("gauge-resource-roles");
    await expect(roles.getByRole("row").filter({ hasText: "Can run" })).toContainText("View, Run");
    await expect(roles.getByRole("row").filter({ hasText: "Can edit" })).toContainText(
      "View, Run, Edit",
    );
    await expect(roles.getByRole("row").filter({ hasText: "Owner" })).toContainText("rp_owners_");
  });

  test("e2e.rag.domain.taxonomy_domain_has_no_card", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto(`/permission/domains/${fixtures.domain_a_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    await expect(page.locator("#gauge-domain-detail-form")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("gauge-resource-bundle-card")).toHaveCount(0);
  });
});

test.describe("e2e.rag.requests", () => {
  test("e2e.rag.requests.resource_label", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "requestor");
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText(`${JOB} · Can view`).first()).toBeVisible({ timeout: 60_000 });

    await page.goto(`/permission/requests/${fixtures.resource_request_id}`, {
      waitUntil: "domcontentloaded",
    });
    await waitForHydrated(page);
    const summary = page.locator("#gauge-request-detail-summary");
    await expect(summary.getByText(`Target: ${JOB} · Can view`)).toBeVisible({ timeout: 60_000 });
    const links = page.getByTestId("gauge-request-resource");
    await expect(links.getByRole("link", { name: "Open in Chronon" })).toHaveAttribute(
      "href",
      "/chronon/jobs/billing-rollup",
    );
    await expect(links.getByRole("link", { name: `Permissions for ${JOB}` })).toHaveAttribute(
      "href",
      `/permission/domains/${fixtures.resource_domain_id}`,
    );
  });

  test("e2e.rag.requests.reviewer_sees_resource_label", async ({ page }) => {
    const { fixtures } = await seedAuth(page, "admin");
    await page.goto("/permission/requests", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByText("Needs Review", { exact: true })).toBeVisible({ timeout: 60_000 });
    await expect(
      page.locator(`a[href="/permission/requests/${fixtures.resource_request_id}"]`).first(),
    ).toBeVisible({ timeout: 60_000 });
    await expect(page.getByText(`${JOB} · Can view`).first()).toBeVisible();
  });
});
