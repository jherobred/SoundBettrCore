import type { Kind } from "./api";

const I = ({ d, children }: { d?: string; children?: React.ReactNode }) => (
  <svg className="i" viewBox="0 0 24 24" aria-hidden="true">
    {d && <path d={d} />}
    {children}
  </svg>
);

export const Buds = () => (
  <I d="M4 15v-3a8 8 0 0 1 16 0v3">
    <rect x="3" y="14" width="4" height="7" rx="1.5" />
    <rect x="17" y="14" width="4" height="7" rx="1.5" />
  </I>
);
export const Wired = () => (
  <I d="M7 10v4a5 5 0 0 0 10 0v-4M12 19v3">
    <circle cx="7" cy="7" r="3" />
    <circle cx="17" cy="7" r="3" />
  </I>
);
export const Speaker = () => (
  <I d="M8 20h8">
    <rect x="3" y="5" width="18" height="11" rx="2" />
  </I>
);
export const Usb = () => (
  <I d="M12 10v6M8 16h8l-1 6H9z">
    <rect x="8" y="2" width="8" height="8" rx="2" />
  </I>
);
export const Devices = () => (
  <I>
    <rect x="4" y="4" width="16" height="7" rx="2" />
    <rect x="4" y="14" width="16" height="7" rx="2" />
  </I>
);
export const Tune = () => (
  <I d="M6 3v18M12 3v18M18 3v18">
    <circle cx="6" cy="14" r="2.2" fill="var(--surface)" />
    <circle cx="12" cy="8" r="2.2" fill="var(--surface)" />
    <circle cx="18" cy="16" r="2.2" fill="var(--surface)" />
  </I>
);
export const Tests = () => <I d="M2 12h4l2-6 4 12 3-9 2 3h5" />;
export const Gear = () => (
  <I d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1">
    <circle cx="12" cy="12" r="3.2" />
  </I>
);

export const kindIcon = (k: Kind) => ({ bluetooth: <Buds />, wired: <Wired />, builtin: <Speaker />, usb: <Usb /> })[k];
