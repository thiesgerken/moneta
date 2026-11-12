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

/// A balance: red only below zero, grey at zero, and otherwise the page's own text colour.
///
/// Money in an account is not money moving. A positive balance is the normal case, so painting
/// every one of them green says nothing, and the red of an overdrawn account is only noticed
/// when it is the one coloured figure in the column. Flows keep `restingAmountColor`.
export const balanceColor = (cents: number) =>
	cents < 0 ? amountColor(cents) : cents === 0 ? 'text-gray-500 dark:text-gray-400' : '';
