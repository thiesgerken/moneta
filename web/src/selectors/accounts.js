import { find } from 'lodash';
import { findUserName } from './users';

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

export const displayAccountById = (accounts, users, userInfo, accountId) => {
  const acc = findAccount(accounts, accountId);
  if (!acc) return `[${accountId}]`;

  if (userInfo && acc.info.userId === userInfo.id) return acc.info.name;

  const userName = findUserName(users, acc.info.userId);
  return `${userName}/${acc.info.name}`;
};

export const displayAccount = (users, userInfo, accountId, account) => {
  if (!account) return `[${accountId}]`;

  if (userInfo && account.info.userId === userInfo.id) return account.info.name;

  const userName = findUserName(users, account.info.userId);
  return `${userName}/${account.info.name}`;
};
