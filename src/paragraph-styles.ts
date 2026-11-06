const headingSizes = [26, 22, 18, 16, 14, 13, 12, 10, 9];

export function headingFontSize(styleId: string): number | null {
  const heading = /^heading\s*([1-9])$/i.exec(styleId);
  return heading ? headingSizes[Number(heading[1]) - 1] : null;
}
