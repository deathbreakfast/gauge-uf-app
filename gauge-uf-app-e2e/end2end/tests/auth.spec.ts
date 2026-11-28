import { test, expect, seedAuth, waitForHydrated } from "./fixtures";

test.describe("e2e.auth", () => {
  test("e2e.auth.anonymous_gate", async ({ page }) => {
    await seedAuth(page, "anonymous");
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("auth-required-empty-state")).toBeAttached({
      timeout: 60_000,
    });
    await expect(page.getByTestId("gauge-overview")).toHaveCount(0);
  });

  test("e2e.auth.unverified_email_gate", async ({ page }) => {
    await seedAuth(page, "unverified");
    await page.goto("/permission", { waitUntil: "domcontentloaded" });
    await waitForHydrated(page);
    await expect(page.getByTestId("gauge-overview")).toHaveCount(0);
    await expect(
      page.getByTestId("email-verification-required-empty-state"),
    ).toBeAttached({ timeout: 30_000 });
  });
});
