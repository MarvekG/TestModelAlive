export function maskKey(value: string) {
  if (value.length <= 10) return "*".repeat(value.length);
  return `${value.slice(0, 7)}...${value.slice(-4)}`;
}
