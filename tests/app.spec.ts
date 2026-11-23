import { test, expect, type Page } from "@playwright/test";

test("preview navigation, preference persistence, and CSV download", async ({
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Dashboard", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Applications", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Enable monitoring" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Usage", exact: true }).click();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export CSV" }).click();
  expect((await download).suggestedFilename()).toMatch(/^NetControl-.*\.csv$/);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByLabel("Appearance").selectOption("dark");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.getByLabel("Data units").selectOption("binary");
  await page.getByLabel("History retention").selectOption("30");
  await page.reload();
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByLabel("Appearance")).toHaveValue("dark");
  await expect(page.getByLabel("Data units")).toHaveValue("binary");
  await expect(page.getByLabel("History retention")).toHaveValue("30");
  await expect(
    page.getByRole("button", { name: "Reset recorded history" }),
  ).toBeDisabled();
});

type TestWindow = Window & {
  isTauri: boolean;
  calls: { cmd: string; args: Record<string, unknown> }[];
  __TAURI_INTERNALS__: {
    invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
  };
};
async function desktopBridge(page: Page) {
  await page.addInitScript(() => {
    const bridge = window as unknown as TestWindow;
    bridge.isTauri = true;
    bridge.calls = [];
    let blocked = false;
    const path = "C:\\NetControlTest\\Example.exe";
    let settings = {
      theme: "light",
      units: "decimal",
      retentionDays: 90,
      adapter: "",
      minimizeToTray: true,
      startMinimized: false,
      launchAtStartup: false,
    };
    bridge.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args = {}) => {
        bridge.calls.push({ cmd, args });
        if (cmd === "get_foundation")
          return { version: "0.1.0", databaseReady: true, settings };
        if (cmd === "get_snapshot")
          return {
            paused: false,
            adapter: "Test adapter",
            adapters: [
              { name: "Test adapter", virtualAdapter: false, online: true },
            ],
            points: [],
            download: 0,
            upload: 0,
            status: "Monitoring",
            attribution: "Disabled",
            applications: [],
            error: "",
          };
        if (cmd === "get_history")
          return {
            days: [],
            applications: [{ path, downloaded: 1024, uploaded: 512 }],
            blockedPaths: blocked ? [path] : [],
            firewallError: "",
          };
        if (cmd === "get_app_history") return [];
        if (cmd === "set_firewall") {
          blocked = Boolean(args.blocked);
          return;
        }
        if (cmd === "save_settings") {
          settings = args.settings as typeof settings;
          return;
        }
        return;
      },
    };
  });
}

test("desktop firewall confirmation sends a scoped command and shows the resulting rule", async ({
  page,
}) => {
  await desktopBridge(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Applications", exact: true }).click();
  await page.getByRole("button", { name: "Block", exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: "Block outbound network access?" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  expect(
    await page.evaluate(() =>
      (window as unknown as TestWindow).calls.filter(
        (c) => c.cmd === "set_firewall",
      ),
    ),
  ).toHaveLength(0);
  await page.getByRole("button", { name: "Block", exact: true }).click();
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(
    page.getByText("Outbound block rule", { exact: true }),
  ).toBeVisible();
  const changes = await page.evaluate(() =>
    (window as unknown as TestWindow).calls.filter(
      (c) => c.cmd === "set_firewall",
    ),
  );
  expect(changes).toEqual([
    {
      cmd: "set_firewall",
      args: { path: "C:\\NetControlTest\\Example.exe", blocked: true },
    },
  ]);
  await page.getByRole("button", { name: "Remove block", exact: true }).click();
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(
    page.getByText("No NetControl rule", { exact: true }),
  ).toBeVisible();
});

for (const scale of [1.25, 1.5, 2, 2.5]) {
  test.describe(`display scale ${scale}`, () => {
    test.use({ deviceScaleFactor: scale });
    test("compact layout and dark theme remain usable", async ({
      page,
    }, info) => {
      await page.setViewportSize({ width: 760, height: 540 });
      await page.goto("/");
      await page.getByRole("button", { name: "Settings", exact: true }).click();
      await page.getByLabel("Appearance").selectOption("dark");
      await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth),
      ).toBeLessThanOrEqual(760);
      await page.screenshot({
        path: info.outputPath("settings.png"),
        fullPage: true,
      });
    });
  });
}
