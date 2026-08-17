export const formatDuration = (durationMs: number, active = false): string => {
  const seconds = Math.max(0, Math.floor(durationMs / 1000));
  if (seconds < 1) return active ? '0s' : '<1s';
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return remainingSeconds > 0 ? `${minutes}m ${remainingSeconds}s` : `${minutes}m`;
};
