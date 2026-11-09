/**
 * Person pickers on Gauge's admin pages: permission grants to people and groups,
 * service users joining a group (and holding permissions through it), people-only
 * owners, domain owners and the permission's owners group. Every scenario runs on a
 * desktop in light mode, a desktop in dark mode and a phone.
 */
import {
  test,
  expect,
  seedAuth,
  waitForHydrated,
  ADMIN_AGENT_NAME,
  LOOKS,
  PEOPLE,
  expectNoA11yViolations,
  pickPrincipal,
  pickerInput,
  pickerRow,
  searchPicker,
  searchPickerAnswered,
  useLook,
  userRow,
  type Look,
} from "./fixtures";

type Page = import("@playwright/test").Page;

const ALLOW = "gauge-perm-allow-picker";
const MEMBERS = "gauge-group-members-picker";
const OWNERS = "gauge-group-owners-picker";
const DOMAIN_OWNERS = "gauge-domain-owners-picker";

function unique(prefix: string, look: Look): string {
  return `${prefix}-${look}-${Date.now()}`;
}

async function createPermission(page: Page, name: string) {
  await page.goto("/permission/create-permission", { waitUntil: "domcontentloaded" });
  await waitForHydrated(page);
  await page.getByLabel(/Display name/i).fill(name);
  await page.getByLabel(/Description/i).fill("person picker e2e");
  await page.locator("select").first().selectOption({ label: "Ops" });
  await page.getByRole("button", { name: /Create Permission/i }).click();
  await expect(page).toHaveURL(/\/permission\/permissions\/[^/]+$/, { timeout: 60_000 });
  await waitForHydrated(page);
  await expect(pickerInput(page, ALLOW)).toBeVisible({ timeout: 60_000 });
}

async function createGroup(page: Page, name: string) {
  await page.goto("/permission/create-group", { waitUntil: "domcontentloaded" });
  await waitForHydrated(page);
  await page.getByLabel(/Display name|Group name/i).first().fill(name);
  await page.getByLabel(/Description/i).fill("person picker e2e");
  await page.getByRole("button", { name: /Create Group/i }).click();
  await expect(page).toHaveURL(/\/permission\/groups\/[^/]+$/, { timeout: 60_000 });
  await waitForHydrated(page);
  await expect(pickerInput(page, MEMBERS)).toBeVisible({ timeout: 60_000 });
}

async function createDomain(page: Page, name: string) {
  await page.goto("/permission/create-domain", { waitUntil: "domcontentloaded" });
  await waitForHydrated(page);
  await page.getByLabel(/Domain name/i).fill(name);
  await page.getByLabel(/Description/i).fill("person picker e2e");
  await page.getByRole("button", { name: "Create Domain", exact: true }).click();
  await expect(page).toHaveURL(/\/permission\/domains\//, { timeout: 60_000 });
  await waitForHydrated(page);
}

function serviceUserRows(list: import("@playwright/test").Locator) {
  return list.locator('[data-testid="person-picker-row"][data-kind="service_user"]');
}

for (const look of LOOKS) {
  test.describe(`pw-gauge-person-picker [${look}]`, () => {
    test.beforeEach(async ({ page }) => {
      test.setTimeout(150_000);
      await seedAuth(page, "admin");
      await useLook(page, look);
    });

    test(`pw-gauge-grant-person-happy [${look}]`, async ({ page }) => {
      await createPermission(page, unique("PP-Person", look));
      const list = await searchPickerAnswered(page, ALLOW, "Rae");
      const rae = pickerRow(list, PEOPLE.requestor);
      await expect(rae).toBeVisible({ timeout: 30_000 });
      await expect(rae).not.toContainText("@");
      await expectNoA11yViolations(page, `permission allow picker open (${look})`);
      await rae.click();
      const allowList = page.locator("#gauge-perm-allow-remove");
      await expect(allowList.getByText(userRow("requestor"))).toBeVisible({ timeout: 60_000 });

      // The person now on the list shows disabled.
      const again = await searchPickerAnswered(page, ALLOW, "Rae");
      await expect(pickerRow(again, PEOPLE.requestor)).toHaveAttribute("data-disabled", "true", {
        timeout: 30_000,
      });
    });

    test(`pw-gauge-grant-person-short-query-sad [${look}]`, async ({ page }) => {
      await createPermission(page, unique("PP-Short", look));
      const list = await searchPicker(page, ALLOW, "R");
      await expect(list.locator('[data-testid="person-picker-row"][data-kind="user"]')).toHaveCount(0);
      await expect(page.locator("#gauge-perm-allow-remove").getByText(userRow("requestor")))
        .toHaveCount(0);
    });

    test(`pw-gauge-grant-group-happy [${look}]`, async ({ page }) => {
      await createPermission(page, unique("PP-Group", look));
      await pickPrincipal(page, ALLOW, "Deployers", "Deployers", true);
      const allowList = page.locator("#gauge-perm-allow-remove");
      await expect(allowList.getByText("Deployers", { exact: true })).toBeVisible({
        timeout: 60_000,
      });

      // A group already on the list shows disabled.
      const again = await searchPicker(page, ALLOW, "Deployers");
      await expect(pickerRow(again, "Deployers", true).first()).toHaveAttribute(
        "data-disabled",
        "true",
        { timeout: 30_000 },
      );
    });

    test(`pw-gauge-grant-service-user-via-group-happy [${look}]`, async ({ page }) => {
      const group = unique("PP-SU", look);
      await createGroup(page, group);

      let list = await searchPickerAnswered(page, MEMBERS, "agent");
      const agent = serviceUserRows(list).filter({ hasText: ADMIN_AGENT_NAME });
      await expect(agent).toContainText("Service user · owned by you", { timeout: 30_000 });
      await expectNoA11yViolations(page, `group members picker open (${look})`);
      await agent.click();
      const members = page.locator("#gauge-group-member-remove");
      await expect(members.getByText("admin-agent", { exact: true })).toBeVisible({
        timeout: 60_000,
      });

      // The service user is a member now, so its row shows disabled.
      list = await searchPickerAnswered(page, MEMBERS, "agent");
      await expect(serviceUserRows(list).filter({ hasText: ADMIN_AGENT_NAME })).toHaveAttribute(
        "data-disabled",
        "true",
        { timeout: 30_000 },
      );

      // The service user holds the permission through the group.
      await createPermission(page, unique("PP-SU-Perm", look));
      await pickPrincipal(page, ALLOW, group, group, true);
      await expect(
        page.locator("#gauge-perm-allow-remove").getByText(group, { exact: true }),
      ).toBeVisible({ timeout: 60_000 });
    });

    test(`pw-gauge-grant-service-user-direct-sad [${look}]`, async ({ page }) => {
      await createPermission(page, unique("PP-SU-Direct", look));
      // Service users get permissions through groups, so the allow list never offers them.
      const list = await searchPickerAnswered(page, ALLOW, "agent");
      await expect(serviceUserRows(list)).toHaveCount(0);
    });

    test(`pw-gauge-super-user-no-service-users-sad [${look}]`, async ({ page }) => {
      await page.goto("/permission/groups/super_user_group", { waitUntil: "domcontentloaded" });
      await waitForHydrated(page);
      // Gauge never lets a service user into the Super User group.
      const list = await searchPickerAnswered(page, MEMBERS, "agent");
      await expect(serviceUserRows(list)).toHaveCount(0);
    });

    test(`pw-gauge-owners-people-only [${look}]`, async ({ page }) => {
      await createGroup(page, unique("PP-Owners", look));

      // Sad: no service users and no groups in the owners picker.
      let list = await searchPickerAnswered(page, OWNERS, "agent");
      await expect(serviceUserRows(list)).toHaveCount(0);
      list = await searchPickerAnswered(page, OWNERS, "Deployers");
      await expect(list.locator('[data-testid="person-picker-row"][data-kind="extra"]')).toHaveCount(0);

      // Happy: a person becomes an owner.
      await pickPrincipal(page, OWNERS, "Rae", PEOPLE.requestor);
      await expect(
        page.locator("#gauge-group-owner-remove").getByText(userRow("requestor")),
      ).toBeVisible({ timeout: 60_000 });
    });

    test(`pw-gauge-domain-owner [${look}]`, async ({ page }) => {
      await createDomain(page, unique("PP-Domain", look));
      await pickPrincipal(page, DOMAIN_OWNERS, "Otto", PEOPLE.outsider);
      await expect(
        page.locator("#gauge-domain-owner-remove").getByText(userRow("outsider")),
      ).toBeVisible({ timeout: 60_000 });

      // An existing owner shows disabled.
      const list = await searchPickerAnswered(page, DOMAIN_OWNERS, "Otto");
      await expect(pickerRow(list, PEOPLE.outsider)).toHaveAttribute("data-disabled", "true", {
        timeout: 30_000,
      });
    });

    test(`pw-gauge-owners-group-picker [${look}]`, async ({ page }) => {
      await createPermission(page, unique("PP-OwnersGroup", look));
      const owners = page.locator("#gauge-perm-owners-group-picker");
      const input = owners.getByPlaceholder(/Search permission groups/i);

      // Sad: a name no group has finds nothing.
      await input.click();
      await input.fill("zz-no-such-group");
      await expect(page.getByRole("listbox").getByText("No results")).toBeVisible({
        timeout: 30_000,
      });

      // Happy: pick a group, save, and it sticks across a reload.
      await input.fill("Deployers");
      const option = page
        .getByRole("listbox")
        .getByRole("option")
        .filter({ has: page.getByText("Deployers", { exact: true }) })
        .first();
      await expect(option).toBeVisible({ timeout: 30_000 });
      await option.click();
      const caption = page.getByTestId("gauge-perm-owners-group");
      await expect(caption).toHaveText("Owned by Deployers", { timeout: 30_000 });
      await page.getByRole("button", { name: /Save Changes/i }).click();
      await expect(page.locator(".orbital-message-bar--error")).toHaveCount(0);
      await page.reload({ waitUntil: "domcontentloaded" });
      await waitForHydrated(page);
      await expect(caption).toHaveText("Owned by Deployers", { timeout: 60_000 });
    });
  });
}
