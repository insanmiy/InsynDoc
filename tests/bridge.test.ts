import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { desktop, watchExternalLinks } from '../src/bridge';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: () => true }));

function packet(metadata: unknown, bytes = new Uint8Array()): ArrayBuffer {
  const header = new TextEncoder().encode(JSON.stringify(metadata));
  const result = new Uint8Array(4 + header.length + bytes.length);
  new DataView(result.buffer).setUint32(0, header.length, true);
  result.set(header, 4);
  result.set(bytes, 4 + header.length);
  return result.buffer;
}

describe('Binary Desktop Bridge', () => {
  it('Downloads Binary Format Copies With A Suggested Name And No Active Token', async () => {
    vi.mocked(invoke).mockResolvedValueOnce('Report.rtf');
    expect(await desktop.download!(new Uint8Array([123, 92, 114]), 'rtf', 'Report.docx')).toBe('Report.rtf');
    const [command, body] = vi.mocked(invoke).mock.calls[0];
    expect(command).toBe('download_document');
    const packet = body as Uint8Array;
    const length = new DataView(packet.buffer).getUint32(0, true);
    expect(JSON.parse(new TextDecoder().decode(packet.subarray(4, 4 + length)))).toEqual({ format: 'rtf', name: 'Report.docx' });
    expect([...packet.subarray(4 + length)]).toEqual([123, 92, 114]);
  });

  it('Uses Native PDF Export And Preserves Save Dialog Cancellation', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(null);
    expect(await desktop.downloadPdf!('Report.docx')).toBeNull();
    expect(invoke).toHaveBeenCalledWith('download_pdf', { name: 'Report.docx' });
  });
  it('Receives A Startup Document Without Opening A File Dialog', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(packet({ token: 'startup', name: 'Downloaded Report.docx' }, new Uint8Array([80, 75, 3, 4])));
    expect(await desktop.openStartup!()).toMatchObject({ token: 'startup', name: 'Downloaded Report.docx', bytes: new Uint8Array([80, 75, 3, 4]) });
    expect(invoke).toHaveBeenCalledWith('open_startup_document');
    vi.mocked(invoke).mockResolvedValueOnce(packet(null));
    expect(await desktop.openStartup!()).toBeNull();
  });
  beforeEach(() => vi.mocked(invoke).mockReset());

  it('Receives A Large Document Without A JSON Number Array', async () => {
    const bytes = new Uint8Array(25 * 1024 * 1024).fill(255);
    vi.mocked(invoke).mockResolvedValue(packet({ token: '7', name: 'Résumé.docx' }, bytes));
    const file = await desktop.open();
    expect(file?.name).toBe('Résumé.docx');
    expect(file?.bytes).toBeInstanceOf(Uint8Array);
    expect(file?.bytes.length).toBe(bytes.length);
    expect(file?.bytes[bytes.length - 1]).toBe(255);
  });

  it('Sends Only The Supplied Byte View With Unicode Metadata', async () => {
    const source = new Uint8Array([99, 80, 75, 0, 255, 88]);
    vi.mocked(invoke).mockResolvedValue({ token: '7', name: 'Résumé.docx' });
    await desktop.save(source.subarray(1, 5), '7', true, 'Résumé.docx');
    const body = vi.mocked(invoke).mock.calls[0][1];
    expect(body).toBeInstanceOf(Uint8Array);
    const raw = body as Uint8Array;
    const length = new DataView(raw.buffer, raw.byteOffset, raw.byteLength).getUint32(0, true);
    expect(JSON.parse(new TextDecoder().decode(raw.subarray(4, 4 + length)))).toEqual({ token: '7', saveAs: true, name: 'Résumé.docx' });
    expect(raw.subarray(4 + length)).toEqual(source.subarray(1, 5));
  });

  it('Handles Cancellation And Recent Binary Documents', async () => {
    vi.mocked(invoke).mockResolvedValueOnce(packet(null));
    expect(await desktop.open()).toBeNull();
    vi.mocked(invoke).mockResolvedValueOnce(packet({ token: '8', name: 'Recent.docx' }, new Uint8Array([80, 75])));
    expect((await desktop.openRecent!('8')).bytes).toEqual(new Uint8Array([80, 75]));
  });

  it('Enforces The Save Size Boundary Before Invoking Native Code', async () => {
    vi.mocked(invoke).mockResolvedValue(null);
    await expect(desktop.save(new Uint8Array(25 * 1024 * 1024), null, false)).resolves.toBeNull();
    expect(vi.mocked(invoke).mock.calls[0][1]).toBeInstanceOf(Uint8Array);
    vi.mocked(invoke).mockClear();
    await expect(desktop.save(new Uint8Array(25 * 1024 * 1024 + 1), null, false)).rejects.toThrow('25 MB');
    expect(invoke).not.toHaveBeenCalled();
  });

  it('Rejects Oversized Responses And Save Metadata', async () => {
    vi.mocked(invoke).mockResolvedValue(packet({ token: '7', name: 'Large.docx' }, new Uint8Array(25 * 1024 * 1024 + 1)));
    await expect(desktop.open()).rejects.toThrow('25 MB');
    vi.mocked(invoke).mockClear();
    await expect(desktop.save(new Uint8Array(), null, false, 'A'.repeat(64 * 1024))).rejects.toThrow('Metadata');
    expect(invoke).not.toHaveBeenCalled();
  });

  it.each([new ArrayBuffer(0), new Uint8Array([255, 255, 255, 255]).buffer, packet({ token: 1, name: 'Bad.docx' }), packet(null, new Uint8Array([1]))])('Rejects Malformed Responses', async (body) => {
    vi.mocked(invoke).mockResolvedValue(body);
    await expect(desktop.open()).rejects.toThrow();
  });
});

describe('Native Hyperlink Routing', () => {
  it('Routes Browser Requests With The Latest Preference And Restores The Webview Handler', async () => {
    const originalOpen = vi.fn((_url?: string | URL) => null);
    const host = { open: originalOpen };
    vi.stubGlobal('window', host);
    vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
    let browserPath: string | null = null;
    const reportError = vi.fn();
    const cleanup = watchExternalLinks(() => browserPath, reportError);
    try {
      host.open('https://example.com/');
      expect(invoke).toHaveBeenLastCalledWith('open_external_link', { url: 'https://example.com/', browserPath: null });
      browserPath = 'C:/Program Files/Browser/browser.exe';
      host.open('https://example.com/next');
      expect(invoke).toHaveBeenLastCalledWith('open_external_link', { url: 'https://example.com/next', browserPath });
      vi.mocked(invoke).mockRejectedValueOnce(new Error('Browser Is Missing.'));
      host.open('https://example.com/');
      await vi.waitFor(() => expect(reportError).toHaveBeenCalledWith('Browser Is Missing.'));
    } finally {
      cleanup();
      expect(host.open).toBe(originalOpen);
      vi.unstubAllGlobals();
    }
  });
});
