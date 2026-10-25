import { defineConfig, devices } from "@playwright/test";

const headed = !!process.env.PW_HEADED;

export default defineConfig({
  testDir: "./tests",
  timeout: 40_000,
  expect: { timeout: 20_000 },
  fullyParallel: false,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["list"]],
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:3150",
    actionTimeout: 20_000,
    navigationTimeout: 32_000,
    ...devices["Desktop Chrome"],
    launchOptions: {
      args: ["--disable-dev-shm-usage"],
      ...(headed
        ? { slowMo: Number(process.env.PW_SLOW_MO ?? 250) }
        : {}),
    },
  },
});
