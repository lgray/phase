import { beforeEach, expect, it, vi } from "vitest";

const { checkMock, isDesktopTauriMock } = vi.hoisted(() => ({
  checkMock: vi.fn(),
  isDesktopTauriMock: vi.fn(),
}));

vi.mock("../../services/platform", () => ({ isDesktopTauri: isDesktopTauriMock }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: checkMock }));

beforeEach(() => {
  vi.clearAllMocks();
  vi.resetModules();
  vi.unstubAllEnvs();
});

it("does not import or invoke the updater on Android/iOS", async () => {
  vi.stubEnv("DEV", false);
  isDesktopTauriMock.mockReturnValue(false);
  const updater = await import("../tauriUpdater");
  updater.registerTauriUpdater();
  expect(updater.checkForTauriUpdate()).toBe(false);
  expect(checkMock).not.toHaveBeenCalled();
});

it("retains desktop updater reachability", async () => {
  vi.stubEnv("DEV", false);
  isDesktopTauriMock.mockReturnValue(true);
  checkMock.mockResolvedValue(null);
  const updater = await import("../tauriUpdater");
  updater.registerTauriUpdater();
  await vi.waitFor(() => expect(checkMock).toHaveBeenCalledOnce());
});

it("does not self-update a dev build", async () => {
  vi.stubEnv("DEV", true);
  isDesktopTauriMock.mockReturnValue(true);
  checkMock.mockResolvedValue(null);
  const updater = await import("../tauriUpdater");
  updater.registerTauriUpdater();
  // An unguarded registration awaits a dynamic import before it calls `check()`.
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(checkMock).not.toHaveBeenCalled();
  expect(updater.checkForTauriUpdate()).toBe(false);
});
