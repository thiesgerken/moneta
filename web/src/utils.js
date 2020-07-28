export const formatAmount = amount => {
  if (amount <= 0) return `${(amount / 100.0).toFixed(2)}€`;

  return `+${(amount / 100.0).toFixed(2)}€`;
};
