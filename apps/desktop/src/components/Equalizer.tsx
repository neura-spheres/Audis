/** Each bar's own speed and starting point, so together they read as sound. */
const BARS = [
  { duration: 0.9, delay: -0.2 },
  { duration: 0.7, delay: -0.5 },
  { duration: 1.05, delay: -0.1 },
  { duration: 0.8, delay: -0.65 },
];

/**
 * The listening indicator: bars that move while Audis hears, and lie still,
 * flat, while it is paused. Motion here always means "listening right now".
 */
export function Equalizer({
  active,
  size = 14,
  colour = "currentColor",
}: {
  active: boolean;
  size?: number;
  colour?: string;
}) {
  const bar = Math.max(2, Math.round(size / 6));

  return (
    <span
      aria-hidden
      className="flex shrink-0 items-end justify-between"
      style={{ width: size, height: size }}
    >
      {BARS.map((timing, index) => (
        <span
          key={index}
          style={{
            width: bar,
            height: "100%",
            borderRadius: bar,
            background: colour,
            transformOrigin: "bottom",
            transform: active ? undefined : "scaleY(0.3)",
            animation: active
              ? `audis-eq ${timing.duration}s ease-in-out ${timing.delay}s infinite`
              : "none",
            transition: "transform var(--spring-smooth-duration) var(--spring-smooth)",
          }}
        />
      ))}
    </span>
  );
}
