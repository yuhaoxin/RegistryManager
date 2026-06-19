const LOCAL_HOSTNAMES = new Set(["localhost", "127.0.0.1", "::1", "[::1]"]);

export function isLocalRegistryUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    if (LOCAL_HOSTNAMES.has(parsed.hostname)) return true;
    if (/^127\.\d+\.\d+\.\d+$/.test(parsed.hostname)) return true;
    return false;
  } catch {
    return url.includes("localhost") || /^https?:\/\/127\.\d+\.\d+\.\d+/.test(url);
  }
}
