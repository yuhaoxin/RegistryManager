export function DeleteGcWarning() {
  return (
    <div className="preflight-item warn" data-testid="delete-gc-warning">
      在服务端 GC 完成前，存储空间可能不会释放。清单删除只会移除 Registry 引用，并将记录标记为 pending_gc。
    </div>
  );
}
