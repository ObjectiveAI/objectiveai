export function ErrorBanner({ code, message }: { code: number; message: unknown }) {
  return (
    <div className="bg-error/10 border-t border-error/30 px-4 py-2 text-error text-xs">
      Error {code}: {JSON.stringify(message)}
    </div>
  );
}
