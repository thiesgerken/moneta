import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { AVAILABILITY_LABELS, AVAILABILITY_ORDER, availabilityColor } from './availability';

describe('availabilityColor', () => {
	it('gives every availability its own colour', () => {
		const colors = AVAILABILITY_ORDER.map(availabilityColor);
		expect(new Set(colors).size).toBe(AVAILABILITY_ORDER.length);
	});

	// the ramp is indexed by position in the order, so anything not in it has no position
	it('falls back to grey for something it has no ramp entry for', () => {
		expect(availabilityColor('WHENEVER' as (typeof AVAILABILITY_ORDER)[number])).toBe('#94a3b8');
	});
});

describe('AVAILABILITY_ORDER', () => {
	it('has a label for every value and nothing else', () => {
		expect(Object.keys(AVAILABILITY_LABELS).toSorted()).toEqual([...AVAILABILITY_ORDER].toSorted());
	});

	/// GraphQL enums arrive as bare strings, so the order the backend declares them in is not
	/// something the client can read off a response — this list is a copy of it, kept by hand,
	/// and the stacked chart is drawn liquid-at-the-bottom on the strength of it.
	///
	/// The generated schema is the other half of that agreement, and it is checked in, so the
	/// copy can be compared against it here rather than by somebody noticing the stack looks
	/// upside down.
	it('is the backend enum, in the backend order', () => {
		const schema = readFileSync('schema.graphql', 'utf8');
		const body = schema.split('enum AccountAvailability {')[1]?.split('}')[0];

		expect(body, 'AccountAvailability is gone from the schema').toBeDefined();
		expect(body!.trim().split(/\s+/)).toEqual(AVAILABILITY_ORDER);
	});
});
