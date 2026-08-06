/** The navbar entries, and with them the page titles: the two must not drift apart. */
export const NAV = [
	{ href: '/', label: 'Dashboard' },
	{ href: '/analysis', label: 'Auswertung' },
	{ href: '/expenses', label: 'Ausgaben' },
	{ href: '/balances', label: 'Kontostände' },
	{ href: '/categories', label: 'Kategorien' },
	{ href: '/accounts', label: 'Konten' },
	{ href: '/rules', label: 'Regeln' },
	{ href: '/deliveries', label: 'Lieferungen' }
] as const;

export function pageTitle(pathname: string): string {
	// prefix match, so that /analysis/2026/07 is still Auswertung; '/' would match everything
	const entry = NAV.find((e) => (e.href === '/' ? pathname === '/' : pathname.startsWith(e.href)));
	const label = entry?.label ?? (pathname.startsWith('/login') ? 'Login' : undefined);

	return label ? `Moneta – ${label}` : 'Moneta';
}
