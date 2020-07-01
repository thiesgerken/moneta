import { find } from 'lodash';

export const findAccount = (accounts, id) => {
  if (accounts.items === null || id === null) return null;
  return find(accounts.items, x => x.info.id === id) || null;
};

export const findAccountByName = (accounts, name) => {
  if (accounts.items === null || name === null) return null;
  return (
    find(
      accounts.items,
      x => x.info.name.toUpperCase().trim() === name.toUpperCase().trim()
    ) || null
  );
};
