export function numberFormatter(): Intl.NumberFormat {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 });
}

export function costFormatter(
  maximumFractionDigits: number = 2
): Intl.NumberFormat {
  return new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: "USD",
    maximumFractionDigits,
  });
}

export function formatBigNumber(
  formatter: Intl.NumberFormat,
  value: number
): string {
  if (value >= 1e9) return formatter.format(value / 1e9) + "B";
  if (value >= 1e6) return formatter.format(value / 1e6) + "M";
  if (value >= 1e3) return formatter.format(value / 1e3) + "K";
  return formatter.format(value);
}
