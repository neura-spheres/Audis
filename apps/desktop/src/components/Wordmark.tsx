/** The Audis mark. */
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
