import { find } from 'lodash';

export const findExpenseQueryData = (expenses, query) => {
  const qjson = JSON.stringify(query);

  return find(expenses.queries, x => qjson === JSON.stringify(x.query)) || null;
};

export default findExpenseQueryData;
