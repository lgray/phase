import { afterEach, describe, expect, it, vi } from "vitest";

import { downloadBlob } from "../fileDownload.ts";

describe("downloadBlob", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    Reflect.deleteProperty(window, "showSaveFilePicker");
    Reflect.deleteProperty(URL, "createObjectURL");
    Reflect.deleteProperty(URL, "revokeObjectURL");
  });

  function stubAnchorDownload() {
    let downloadedBlob: Blob | null = null;
    let downloadedName: string | null = null;
    Object.defineProperty(URL, "createObjectURL", {
      configurable: true,
      writable: true,
      value: vi.fn((blob: Blob) => {
        downloadedBlob = blob;
        return "blob:mock-url";
      }),
    });
    Object.defineProperty(URL, "revokeObjectURL", {
      configurable: true,
      writable: true,
      value: vi.fn(),
    });
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
      this: HTMLAnchorElement,
    ) {
      downloadedName = this.download;
    });
    return {
      blob: () => downloadedBlob,
      filename: () => downloadedName,
    };
  }

  it("writes through the save picker when it succeeds", async () => {
    const write = vi.fn(async () => {});
    const close = vi.fn(async () => {});
    const showSaveFilePicker = vi.fn(async () => ({
      createWritable: async () => ({ write, close }),
    }));
    Object.defineProperty(window, "showSaveFilePicker", {
      configurable: true,
      value: showSaveFilePicker,
    });
    const blob = new Blob(["data"], { type: "text/plain" });

    const filename = await downloadBlob("notes.txt", blob, [
      { description: "Text", accept: { "text/plain": [".txt"] } },
    ]);

    expect(filename).toBe("notes.txt");
    expect(showSaveFilePicker).toHaveBeenCalledWith({
      suggestedName: "notes.txt",
      types: [{ description: "Text", accept: { "text/plain": [".txt"] } }],
    });
    expect(write).toHaveBeenCalledWith(blob);
    expect(close).toHaveBeenCalledOnce();
  });

  it("falls back to an anchor download when the picker fails", async () => {
    // Chrome exposes showSaveFilePicker but the picker path can fail there;
    // the download must then degrade to the plain anchor path Firefox uses.
    Object.defineProperty(window, "showSaveFilePicker", {
      configurable: true,
      value: vi.fn(async () => {
        throw new DOMException("The picker is unavailable", "SecurityError");
      }),
    });
    const anchor = stubAnchorDownload();
    const blob = new Blob(["data"], { type: "text/plain" });

    const filename = await downloadBlob("notes.txt", blob);

    expect(filename).toBe("notes.txt");
    expect(anchor.filename()).toBe("notes.txt");
    expect(anchor.blob()).toBe(blob);
  });

  it("does not download when the user cancels the picker", async () => {
    Object.defineProperty(window, "showSaveFilePicker", {
      configurable: true,
      value: vi.fn(async () => {
        throw new DOMException("The user aborted a request", "AbortError");
      }),
    });
    const clickSpy = vi
      .spyOn(HTMLAnchorElement.prototype, "click")
      .mockImplementation(() => {});

    const err = await downloadBlob("notes.txt", new Blob(["data"])).catch((e: unknown) => e);

    expect(err).toBeInstanceOf(DOMException);
    expect((err as DOMException).name).toBe("AbortError");
    expect(clickSpy).not.toHaveBeenCalled();
  });
});
