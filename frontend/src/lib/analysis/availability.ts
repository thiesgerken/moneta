import type { AccountAvailability$options } from '$houdini';

export const AVAILABILITY_LABELS: Record<AccountAvailability$options, string> = {
	IMMEDIATELY: 'Sofort',
	DAYS: 'Tage',
	WEEKS: 'Wochen',
	MONTHS: 'Monate',
	YEARS: 'Jahre',
	DECADES: 'Jahrzehnte'
};

/// Most liquid first, matching the backend enum. GraphQL enums arrive as bare strings, so the
/// order the values are declared in is not something the client can read off them.
export const AVAILABILITY_ORDER: AccountAvailability$options[] = [
	'IMMEDIATELY',
	'DAYS',
	'WEEKS',
	'MONTHS',
	'YEARS',
	'DECADES'
];

/// Availability has no colour of its own the way an account does, so it gets a ramp that runs
/// warm to cool along the order above -- liquid at the bottom of a stack, illiquid at the top.
const RAMP = ['#f05a28', '#f0a428', '#7db35b', '#4a90c2', '#8e6cbf', '#bf6c8e'];

export function availabilityColor(availability: AccountAvailability$options): string {
	return RAMP[AVAILABILITY_ORDER.indexOf(availability)] ?? '#94a3b8';
}
