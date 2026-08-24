import { describe, expect, it } from 'vitest';
import {
	categoryColor,
	deltaColor,
	errorClass,
	integrationError,
	periodLabel,
	periodTicks
} from './format';
import { grainFor } from './state.svelte';

describe('integrationError', () => {
	// the gap is what says an expense is missing, and it is a distance rather than a direction
	it('is the distance between what was measured and what was integrated', () => {
		expect(integrationError({ endTotal: 10000, integratedEndTotal: 9500 })).toBe(500);
		expect(integrationError({ endTotal: 9500, integratedEndTotal: 10000 })).toBe(500);
	});

	it('is zero when the two agree', () => {
		expect(integrationError({ endTotal: 10000, integratedEndTotal: 10000 })).toBe(0);
	});

	/// On the accrual axis there is nothing to integrate — a share of a subscription is a cost
	/// the month carried, not money that left an account — so the column has no answer rather
	/// than an answer of zero, which would read as "checked, and fine".
	it('has no answer where the backend had nothing to integrate', () => {
		expect(integrationError({ endTotal: 10000, integratedEndTotal: null })).toBeNull();
	});
});

describe('errorClass', () => {
	// quiet until it is big enough to mean a missing expense
	it('stays grey for the cents that rounding explains', () => {
		expect(errorClass(0)).toContain('gray');
		expect(errorClass(2499)).toContain('gray');
	});

	it('warns from 25 € and shouts from 50 €', () => {
		expect(errorClass(2500)).toContain('yellow');
		expect(errorClass(4999)).toContain('yellow');
		expect(errorClass(5000)).toContain('red');
	});
});

describe('deltaColor', () => {
	it('is red down, green up and grey for nothing at all', () => {
		expect(deltaColor(-1)).toBe('red');
		expect(deltaColor(1)).toBe('green');
		expect(deltaColor(0)).toBe('gray');
	});
});

describe('periodLabel', () => {
	// the backend dates a period at midnight UTC, so the label has to read it there: an hour
	// behind, "1. Januar" is the 31st of December and the year under the bar is the wrong one
	const start = (y: number, m: number) => new Date(Date.UTC(y, m - 1, 1));

	it('is short enough to sit under a bar', () => {
		expect(periodLabel(start(2026, 8), 'MONTH')).toBe('Aug. 26');
	});

	// `26` under a bar a year wide reads as a day of the month
	it('spells a year out in full', () => {
		expect(periodLabel(start(2026, 1), 'YEAR')).toBe('2026');
	});

	it('names the year a period starts in, not the one the reader is standing in', () => {
		expect(periodLabel(start(2019, 1), 'YEAR')).toBe('2019');
		expect(periodLabel(start(2019, 12), 'MONTH')).toBe('Dez. 19');
	});
});

describe('periodTicks', () => {
	/// A category axis draws one label per bar unless told otherwise, which at ten years of
	/// months is 120 of them.
	it('stops the labels turning into a smear', () => {
		expect(periodTicks(120)).toBe(12);
	});

	it('leaves a short range alone rather than inventing ticks', () => {
		expect(periodTicks(3)).toBe(3);
		expect(periodTicks(0)).toBe(0);
	});
});

describe('grainFor', () => {
	// a range asks for the grain it can be drawn at; the toggle only has to override it
	it('draws a few years as the months they are made of', () => {
		expect(grainFor(1)).toBe('MONTH');
		expect(grainFor(12)).toBe('MONTH');
		expect(grainFor(36)).toBe('MONTH');
	});

	it('and anything longer as years, where the months stop being labelable', () => {
		expect(grainFor(37)).toBe('YEAR');
		expect(grainFor(120)).toBe('YEAR');
	});
});

describe('categoryColor', () => {
	it('uses the colour the category was given', () => {
		expect(categoryColor({ name: 'Lebensmittel', color: '#ff0000' })).toBe('#ff0000');
	});

	it('has one grey for the bucket that is not a category at all', () => {
		expect(categoryColor(null)).toBe('#94a3b8');
	});

	// derived from the name, so a category with no colour at least keeps the same one between
	// renders instead of flickering through the palette
	it('gives an uncoloured category a stable colour of its own', () => {
		const first = categoryColor({ name: 'Haushalt', color: null });
		expect(categoryColor({ name: 'Haushalt' })).toBe(first);
		expect(first).toMatch(/^#[0-9a-f]{6}$/);
	});

	it('does not give two different names the same answer by construction', () => {
		const names = ['Haushalt', 'Lebensmittel', 'Mobilität', 'Gesundheit', 'Freizeit'];
		const colors = names.map((name) => categoryColor({ name }));
		expect(new Set(colors).size).toBeGreaterThan(1);
	});

	// an empty name still has to produce something drawable rather than undefined
	it('answers for a category with no name', () => {
		expect(categoryColor({ name: '' })).toMatch(/^#[0-9a-f]{6}$/);
	});
});
