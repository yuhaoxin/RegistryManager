export interface StaleCacheBannerProps {
  message?: string;
}

export function StaleCacheBanner({ message = "Registry is offline. Showing stale cached data." }: StaleCacheBannerProps) {
  return (
    <div className="preflight-item warn" role="status" data-testid="stale-cache-banner">
      ⚠️ {message}
    </div>
  );
}
