const paths = {
  document: 'M6 3h8l4 4v14H6z M14 3v5h4 M9 12h6 M9 15h6 M9 18h4',
  edit: 'm4 16 12-12 4 4L8 20H4z M14 6l4 4',
  search: 'M21 21l-5-5 M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0',
  outline: 'M8 5h13 M8 12h13 M8 19h9 M3 5h.01 M3 12h.01 M3 19h.01',
  save: 'M5 3h13l3 3v15H3V3z M7 3v6h10V3 M7 21v-8h10v8',
  download: 'M12 3v12 M7 10l5 5 5-5 M4 16v5h16v-5',
  check: 'M5 12l4 4L19 6',
  close: 'M6 6l12 12 M18 6L6 18',
  chevronDown: 'M6 9l6 6 6-6',
  arrowUp: 'M6 14l6-6 6 6',
  arrowDown: 'M6 10l6 6 6-6',
  focus: 'M8 3H3v5 M16 3h5v5 M21 16v5h-5 M3 16v5h5',
  moon: 'M20.9 13A9 9 0 0 1 11 3.1 9 9 0 1 0 20.9 13z',
  sun: 'M12 3V1 M12 23v-2 M3 12H1 M23 12h-2 M4.2 4.2l1.4 1.4 M18.4 18.4l1.4 1.4 M19.8 4.2l-1.4 1.4 M5.6 18.4l-1.4 1.4 M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0',
  monitor: 'M3 4h18v13H3z M8 21h8 M12 17v4',
  page: 'M5 2h14v20H5z M8 7h8 M8 11h8 M8 15h5',
  folder: 'M3 6V4h7l2 3h9v13H3z',
  info: 'M12 11v6 M12 7h.01 M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0'
};

export type IconName = keyof typeof paths | 'settings';

export function Icon({ name, size = 20 }: { name: IconName; size?: number }) {
  if (name === 'settings') return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7"><path d="m9.5 3-.6 2.1-1.8 1-2.1-.5-2.5 4.3L4 11.5v2L2.5 15 5 19.3l2.1-.5 1.8 1 .6 2.2h5l.6-2.2 1.8-1 2.1.5 2.5-4.3-1.5-1.5v-2l1.5-1.6L19 5.6l-2.1.5-1.8-1L14.5 3z"/><circle cx="12" cy="12.5" r="3.2"/></svg>;
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"><path d={paths[name]} /></svg>;
}
