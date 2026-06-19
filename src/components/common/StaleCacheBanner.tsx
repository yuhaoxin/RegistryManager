export interface StaleCacheBannerProps {
  message?: string;
}

export function StaleCacheBanner({ message = "Registry 离线。正在显示已过期的缓存数据。" }: StaleCacheBannerProps) {
  return (
    <div className="preflight-item warn" role="status" data-testid="stale-cache-banner">
      ⚠️ {message}
    </div>
  );
}
