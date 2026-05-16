export function numberIsEmpty(value: number | null | undefined): boolean {
  return value === null || value === undefined || value === 0;
}
