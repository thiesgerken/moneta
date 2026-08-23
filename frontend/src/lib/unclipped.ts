/// Where a calendar of this size goes when it belongs under a field at `field`.
///
/// Below the field, unless that would run off the bottom of the window, in which case above
/// it; left-aligned with the field, unless that would run off the right, in which case as far
/// right as it fits. Never off the top or the left: a box you cannot reach is worse than one
/// overlapping the field it belongs to.
export function place(
	field: { top: number; bottom: number; left: number },
	size: { width: number; height: number },
	window: { width: number; height: number },
	gap = 4
) {
	const below = field.bottom + gap;
	const fits = below + size.height + gap <= window.height;
	return {
		top: fits ? below : Math.max(gap, field.top - size.height - gap),
		left: Math.max(gap, Math.min(field.left, window.width - size.width - gap))
	};
}

/// Lets a `Datepicker`'s calendar hang outside whatever is scrolling around it.
///
/// flowbite draws the calendar `position: absolute` inside the field, so any ancestor that
/// scrolls -- a modal body is one -- trims it at its own edges. In the expense modal that cost
/// the weekend column and the last two weeks of the month: days you could see were not there
/// to be clicked. A fixed box is laid out against the viewport instead, which no ancestor's
/// overflow reaches.
///
/// ponytail: a MutationObserver and `place` rather than a positioning library. Enough for a
/// box that only has to stay on the screen; anything that has to point at what opened it, or
/// dodge its siblings, wants floating-ui.
export function unclipped(wrapper: HTMLElement) {
	const position = () => {
		const calendar = wrapper.querySelector<HTMLElement>('#datepicker-dropdown');
		if (!calendar) return;

		// before measuring: an absolute box may have been squeezed by the column it sat in
		calendar.style.position = 'fixed';
		calendar.style.margin = '0';

		const { top, left } = place(wrapper.getBoundingClientRect(), calendar.getBoundingClientRect(), {
			width: innerWidth,
			height: innerHeight
		});
		calendar.style.top = `${top}px`;
		calendar.style.left = `${left}px`;
	};

	// `childList` only: `position` writes styles, which are attribute changes, so it cannot
	// see its own work and loop. The month grid swapping for the month picker is a childList
	// change too, which is what keeps the taller of the two on the screen.
	const opened = new MutationObserver(position);
	opened.observe(wrapper, { childList: true, subtree: true });
	// capturing, so that the modal body scrolling counts and not only the window
	addEventListener('scroll', position, true);
	addEventListener('resize', position);

	return {
		destroy() {
			opened.disconnect();
			removeEventListener('scroll', position, true);
			removeEventListener('resize', position);
		}
	};
}
