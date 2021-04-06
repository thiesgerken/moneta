import { find } from 'lodash';
import * as moment from 'moment';

export const findExpenseQueryData = (expenses, query) => {
  const qjson = JSON.stringify(query);

  return find(expenses.queries, x => qjson === JSON.stringify(x.query)) || null;
};

export const formatBooking = (expense, xsDown) => {
  const st = moment(expense.info.bookingStart);
  const en = moment(expense.info.bookingEnd);

  if (xsDown) return st.format('L');

  if (st.isSame(en)) return st.format('L LT');
  if (st.isSame(en, 'day')) return `${st.format('L LT')} - ${en.format('LT')}`;
  return `${st.format('L')} - ${en.format('L')}`;
};
