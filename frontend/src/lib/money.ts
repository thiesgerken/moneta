/// Money, formatted the one way the app formats money.
///
/// Everything travels as integer cents; only this divides by 100.

/// `1.234,56 €`, as everywhere a figure is read rather than checked against a sign.
export const euro = (cents: number) =>
	(cents / 100).toLocaleString('de-DE', { style: 'currency', currency: 'EUR' });

/// The same, with the `+` spelled out on anything above zero.
///
/// For the columns where both directions occur: a minus is loud and a positive figure beside it
/// is silent, so "5,99" in a column of "-5,99" reads as a number someone forgot to sign rather
/// than as money that came in. `Intl` writes the minus itself, so only the plus is added here.
export const signedEuro = (cents: number) => (cents > 0 ? '+' : '') + euro(cents);

/// Red for money going out, green for money coming in — one pair, so a figure is recognised
/// before it is read.
export const amountColor = (cents: number) =>
	cents < 0 ? 'text-red-600 dark:text-red-400' : 'text-green-700 dark:text-green-400';

/// The same, but zero stays grey: where "nothing moved" is a real answer, a green 0,00 € claims
/// otherwise.
export const restingAmountColor = (cents: number) =>
	cents === 0 ? 'text-gray-500 dark:text-gray-400' : amountColor(cents);

/// A zero still gets its own cell: it can describe a transfer rather than a missing amount.
export const amountTint = (cents: number) =>
	cents < 0
		? 'bg-red-100 text-red-700 dark:bg-red-950/50 dark:text-red-300'
		: cents > 0
			? 'bg-green-100 text-green-700 dark:bg-green-950/50 dark:text-green-300'
			: 'bg-gray-200 text-gray-600 dark:bg-gray-700 dark:text-gray-200';
