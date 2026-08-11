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
