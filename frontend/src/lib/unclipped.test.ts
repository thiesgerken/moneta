import { describe, expect, it } from 'vitest';
import { place } from './unclipped';

describe('place', () => {
	const window = { width: 1280, height: 800 };
	const calendar = { width: 288, height: 340 };

	it('sits under the field it belongs to', () => {
		expect(place({ top: 180, bottom: 220, left: 400 }, calendar, window)).toEqual({
			top: 224,
			left: 400
		});
	});

	it('flips above the field rather than off the bottom', () => {
		expect(place({ top: 600, bottom: 640, left: 400 }, calendar, window)).toEqual({
			top: 256,
			left: 400
		});
	});

	it('slides left rather than off the right -- the weekend column stays', () => {
		expect(place({ top: 180, bottom: 220, left: 1100 }, calendar, window).left).toBe(988);
	});

	it('stays on the screen where it does not fit at all', () => {
		const tall = { width: 288, height: 900 };
		expect(place({ top: 600, bottom: 640, left: 400 }, tall, window).top).toBe(4);
		const wide = { width: 1400, height: 340 };
		expect(place({ top: 180, bottom: 220, left: 400 }, wide, window).left).toBe(4);
	});
});
