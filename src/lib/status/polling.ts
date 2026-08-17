type PollingKey = string;

type PollingEntry = {
  subscribers: number;
  timer: number;
};

export type PollingRelease = () => void;

export function createPollingCoordinator(
  refresh: (key: string | null) => Promise<void>,
  intervalMs = 1000,
) {
  const entries = new Map<PollingKey, PollingEntry>();

  const subscribe = (key: string | null): PollingRelease => {
    if (!key) {
      void refresh(null);
      return () => undefined;
    }

    const current = entries.get(key);
    if (current) {
      current.subscribers += 1;
    } else {
      void refresh(key);
      entries.set(key, {
        subscribers: 1,
        timer: window.setInterval(() => void refresh(key), intervalMs),
      });
    }

    let released = false;
    return () => {
      if (released) return;
      released = true;
      const entry = entries.get(key);
      if (!entry) return;
      entry.subscribers -= 1;
      if (entry.subscribers > 0) return;
      window.clearInterval(entry.timer);
      entries.delete(key);
    };
  };

  return { subscribe };
}
