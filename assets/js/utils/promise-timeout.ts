export class TimeoutError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TimeoutError";
  }
}

export async function promiseTimeout<T>(
  promise: Promise<T>,
  ms: number,
  operation = "Operation",
): Promise<T> {
  const abort = AbortSignal.timeout(ms);
  const errorMsg = `${operation} timed out after ${ms} ms`;
  return await new Promise<T>((resolve, reject) => {
    if (abort.aborted) {
      reject(new TimeoutError(errorMsg));
      return;
    }

    abort.addEventListener("abort", () => {
      reject(new TimeoutError(errorMsg));
    });

    promise.then(resolve, reject);
  });
}
