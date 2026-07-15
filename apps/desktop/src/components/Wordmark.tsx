/**
 * The Audis mark.
 *
 * Loaded from `public/logo.png` via `BASE_URL` rather than a hardcoded `/`
 * path, so the reference resolves correctly under Vite's relative `base`
 * (`vite.config.ts`) both in the dev server and inside the packaged Tauri app.
 */
export function Wordmark({ size = 32 }: { size?: number }) {
  return (
    <img
      src={`${import.meta.env.BASE_URL}logo.png`}
      alt="Audis"
      width={size}
      height={size}
      style={{ width: size, height: size, borderRadius: size * 0.22 }}
    />
  );
}
