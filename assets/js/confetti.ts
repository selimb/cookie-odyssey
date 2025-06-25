import canvasConfetti from "canvas-confetti";

function randomInRange(min: number, max: number): number {
  return Math.random() * (max - min) + min;
}

export function fireConfetti(): void {
  // Implementation 100% from https://www.kirilv.com/canvas-confetti/#fireworks
  // eslint-disable-next-line no-console -- This is OK
  console.info("Confetti!");

  const duration = 15 * 1000;
  const animationEnd = Date.now() + duration;
  const defaults = {
    startVelocity: 30,
    spread: 360,
    ticks: 60,
    zIndex: 0,
  };

  const interval = setInterval(function () {
    const timeLeft = animationEnd - Date.now();

    if (timeLeft <= 0) {
      clearInterval(interval);
      return;
    }

    const particleCount = 50 * (timeLeft / duration);
    // since particles fall down, start a bit higher than random
    void canvasConfetti({
      ...defaults,
      particleCount,
      origin: { x: randomInRange(0.1, 0.3), y: Math.random() - 0.2 },
    });
    void canvasConfetti({
      ...defaults,
      particleCount,
      origin: { x: randomInRange(0.7, 0.9), y: Math.random() - 0.2 },
    });
  }, 250);
}

// Keep in sync with [confetti-evt]
const CONFETTI_EVT = "app.confetti";
document.body.addEventListener(CONFETTI_EVT, fireConfetti);
