import { find } from 'lodash';

export const findBalanceQueryData = (balances, query) => {
  const qjson = JSON.stringify(query);

  return find(balances.queries, x => qjson === JSON.stringify(x.query)) || null;
};
