interface FileSystemWritableFileStream {
  write: (data: Blob) => Promise<void>;
  close: () => Promise<void>;
}

interface FileSystemFileHandle {
  createWritable: () => Promise<FileSystemWritableFileStream>;
}

interface SaveFilePickerOptions {
  suggestedName?: string;
  types?: Array<{
    description: string;
    accept: Record<string, string[]>;
  }>;
}

type WindowWithSaveFilePicker = Window & {
  showSaveFilePicker?: (options?: SaveFilePickerOptions) => Promise<FileSystemFileHandle>;
};

/**
 * Save a blob to disk, preferring the File System Access "save as" picker
 * when the browser offers one. The picker path is known to fail in Chrome
 * while a plain download succeeds, so any picker failure other than the
 * user cancelling falls back to an anchor download.
 *
 * Returns the filename. Rejects with an AbortError when the user cancels
 * the picker (callers treat that as "no download", not a failure).
 */
export async function downloadBlob(
  filename: string,
  blob: Blob,
  pickerTypes?: SaveFilePickerOptions["types"],
): Promise<string> {
  const saveFilePicker = (window as WindowWithSaveFilePicker).showSaveFilePicker;
  if (saveFilePicker) {
    try {
      const options: SaveFilePickerOptions = { suggestedName: filename };
      if (pickerTypes) options.types = pickerTypes;
      const handle = await saveFilePicker(options);
      const writable = await handle.createWritable();
      await writable.write(blob);
      await writable.close();
      return filename;
    } catch (err) {
      // A cancelled picker is deliberate: the user chose not to save, so do
      // not surprise them with a fallback download.
      if (err instanceof DOMException && err.name === "AbortError") throw err;
      // Otherwise the picker itself is broken in this browser: fall through
      // to the anchor download below.
    }
  }

  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  document.body.removeChild(anchor);
  URL.revokeObjectURL(url);
  return filename;
}
