// 선 아이콘 (SF Symbols 대신). currentColor 를 따른다.
const S = 'fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"';

export const ICONS = {
  logo: `<svg viewBox="0 0 18 18" ${S}><circle cx="9" cy="1.5" r="1.1" fill="currentColor" stroke="none"/><path d="M9 2.6v1"/><rect x="3.2" y="3.6" width="11.6" height="6.6" rx="2.4"/><circle cx="6.7" cy="6.9" r="1" fill="currentColor" stroke="none"/><circle cx="11.3" cy="6.9" r="1" fill="currentColor" stroke="none"/><circle cx="9" cy="11.9" r="1.5"/><path d="M9 13.4v4M9 15.3h1.9M9 16.9h1.5"/></svg>`,
  lock: `<svg viewBox="0 0 12 12" ${S}><rect x="2" y="5.2" width="8" height="5.6" rx="1.2" fill="currentColor"/><path d="M4 5.2V3.8a2 2 0 0 1 4 0v1.4"/></svg>`,
  lockOpen: `<svg viewBox="0 0 12 12" ${S}><rect x="2" y="5.2" width="8" height="5.6" rx="1.2" fill="currentColor"/><path d="M4 5.2V3.8a2 2 0 0 1 3.9-.6"/></svg>`,
  cpu: `<svg viewBox="0 0 12 12" ${S}><rect x="3" y="3" width="6" height="6" rx="1"/><path d="M4.5 1v2M7.5 1v2M4.5 9v2M7.5 9v2M1 4.5h2M1 7.5h2M9 4.5h2M9 7.5h2"/></svg>`,
  eye: `<svg viewBox="0 0 16 16" ${S}><path d="M1.5 8s2.4-4.5 6.5-4.5S14.5 8 14.5 8 12.1 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/></svg>`,
  eyeSlash: `<svg viewBox="0 0 16 16" ${S}><path d="M1.5 8s2.4-4.5 6.5-4.5S14.5 8 14.5 8 12.1 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/><path d="M2.5 13.5l11-11"/></svg>`,
  copy: `<svg viewBox="0 0 16 16" ${S}><rect x="5" y="5" width="8.5" height="8.5" rx="1.6"/><path d="M11 5V3.6A1.1 1.1 0 0 0 9.9 2.5H3.6A1.1 1.1 0 0 0 2.5 3.6v6.3A1.1 1.1 0 0 0 3.6 11H5"/></svg>`,
  pencil: `<svg viewBox="0 0 16 16" ${S}><path d="M10.5 2.8l2.7 2.7L6 12.7l-3.2.5.5-3.2z"/><path d="M2.5 14.5h11"/></svg>`,
  trash: `<svg viewBox="0 0 16 16" ${S}><path d="M2.5 4h11M6 4V2.8h4V4M4 4l.7 9.2h6.6L12 4M6.7 6.5v4.5M9.3 6.5v4.5"/></svg>`,
  plus: `<svg viewBox="0 0 16 16" ${S}><circle cx="8" cy="8" r="6.5" fill="currentColor" stroke="none"/><path d="M8 5v6M5 8h6" stroke="#fff"/></svg>`,
  checklist: `<svg viewBox="0 0 16 16" ${S}><path d="M2 4l1.3 1.3L5.6 3M2 10l1.3 1.3L5.6 9M8 4.2h6M8 10.2h6"/></svg>`,
  refresh: `<svg viewBox="0 0 16 16" ${S}><path d="M13 8a5 5 0 1 1-1.5-3.6"/><path d="M13 2.5v3h-3"/></svg>`,
  gear: `<svg viewBox="0 0 16 16" ${S}><circle cx="8" cy="8" r="2.2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"/></svg>`,
};
