export const numberFormatter = new Intl.NumberFormat();
export const roundedNumberFormatter = new Intl.NumberFormat(undefined, {
  maximumSignificantDigits: 3,
  notation: "compact",
});
export function humanizeFileSize(sizeBytes: number, space = false): string {
  const i = sizeBytes === 0 ? 0 : Math.floor(Math.log(sizeBytes) / Math.log(1000));
  return (sizeBytes / Math.pow(1000, i)).toFixed(1) + (space ? " " : "") + ["B", "KB", "MB", "GB", "TB"][i];
}
