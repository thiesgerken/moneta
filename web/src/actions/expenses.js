import { map, sortBy, zip } from 'lodash';
import { findExpenseQueryData } from '../selectors/expenses';

export const FETCH_EXPENSE_INFO_REQUEST = 'FETCH_EXPENSE_INFO_REQUEST';
export const fetchExpenseInfoRequest = () => ({
  type: FETCH_EXPENSE_INFO_REQUEST,
});

export const FETCH_EXPENSE_INFO_SUCCESS = 'FETCH_EXPENSE_INFO_SUCCESS';
export const fetchExpenseInfoSuccess = json => ({
  type: FETCH_EXPENSE_INFO_SUCCESS,
  data: json,
  receivedAt: Date.now(),
});

export const FETCH_EXPENSE_INFO_ERROR = 'FETCH_EXPENSE_INFO_ERROR';
export const fetchExpenseInfoError = e => ({
  type: FETCH_EXPENSE_INFO_ERROR,
  error: e,
  receivedAt: Date.now(),
});

export const FETCH_EXPENSES_REQUEST = 'FETCH_EXPENSES_REQUEST';
export const fetchExpensesRequest = query => ({
  type: FETCH_EXPENSES_REQUEST,
  query,
});

export const FETCH_EXPENSES_SUCCESS = 'FETCH_EXPENSES_SUCCESS';
export const fetchExpensesSuccess = (query, json) => ({
  type: FETCH_EXPENSES_SUCCESS,
  query,
  data: json,
  receivedAt: Date.now(),
});

export const FETCH_EXPENSES_ERROR = 'FETCH_EXPENSES_ERROR';
export const fetchExpensesError = (query, e) => ({
  type: FETCH_EXPENSES_ERROR,
  error: e,
  query,
  receivedAt: Date.now(),
});

export const INVALIDATE_ALL_EXPENSE_DATA = 'INVALIDATE_ALL_EXPENSE_DATA';
export const invalidateAllExpenseData = () => ({
  type: INVALIDATE_ALL_EXPENSE_DATA,
});

export const INVALIDATE_EXPENSE_INFO = 'INVALIDATE_EXPENSE_INFO';
export const invalidateExpenseInfo = () => ({
  type: INVALIDATE_EXPENSE_INFO,
});

export const INVALIDATE_EXPENSES = 'INVALIDATE_EXPENSES';
export const invalidateExpenses = query => ({
  type: INVALIDATE_EXPENSES,
  query,
});

export const MODIFY_EXPENSE_REQUEST = 'MODIFY_EXPENSE_REQUEST';
export const modifyExpenseRequest = (id, method) => ({
  type: MODIFY_EXPENSE_REQUEST,
  id,
  method,
});

export const MODIFY_EXPENSE_SUCCESS = 'MODIFY_EXPENSE_SUCCESS';
export const modifyExpenseSuccess = (id, method) => ({
  type: MODIFY_EXPENSE_SUCCESS,
  id,
  method,
  receivedAt: Date.now(),
});

export const MODIFY_EXPENSE_ERROR = 'MODIFY_EXPENSE_ERROR';
export const modifyExpenseError = (id, method, error) => ({
  type: MODIFY_EXPENSE_ERROR,
  id,
  method,
  error,
  receivedAt: Date.now(),
});

const preprocessExpense = e => {
  let accountsByAmount = {};

  zip(e.transactions, e.calculatedAmounts).forEach(([t, amount]) => {
    if (!(t.accountId in accountsByAmount)) accountsByAmount[t.accountId] = 0;
    accountsByAmount[t.accountId] += amount;
  });

  accountsByAmount = sortBy(
    map(accountsByAmount, (amount, aid) => ({
      accountId: parseInt(aid, 10),
      amount,
    })),
    [x => -Math.abs(x.amount), x => x.accountId]
  );

  return {
    ...e,
    meta: {
      accountsByAmount,
    },
  };
};

export const fetchExpenses = query => dispatch => {
  dispatch(fetchExpensesRequest(query));
  return fetch('/api/expenses/query', {
    method: 'POST',
    body: JSON.stringify(query),
  })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      res.records = map(res.records, preprocessExpense);
      dispatch(fetchExpensesSuccess(query, res));
    })
    .catch(e => dispatch(fetchExpensesError(query, e)));
};

export const fetchExpenseInfo = () => dispatch => {
  dispatch(fetchExpenseInfoRequest());
  return fetch('/api/expenses/info')
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(fetchExpenseInfoSuccess(res));
    })
    .catch(e => dispatch(fetchExpenseInfoError(e)));
};

export const updateExpenseInfo = () => (dispatch, getState) => {
  const { expenses } = getState();

  if (
    !expenses.info.didInvalidate &&
    expenses.info.lastUpdated !== null &&
    Date.now() - expenses.info.lastUpdated > 60 * 60 * 1e3
  )
    dispatch(invalidateExpenseInfo());

  if (
    !expenses.info.isFetching &&
    ((expenses.info.lastUpdated === null &&
      expenses.info.fetchError === null) ||
      expenses.info.didInvalidate)
  )
    return dispatch(fetchExpenseInfo());

  return Promise.resolve();
};

export const updateExpenses = query => (dispatch, getState) => {
  const { expenses } = getState();

  const q = findExpenseQueryData(expenses, query);
  if (
    q !== null &&
    !q.didInvalidate &&
    q.lastUpdated !== null &&
    Date.now() - q.lastUpdated > 60 * 60 * 1e3
  )
    dispatch(invalidateExpenses(query));

  if (
    q === null ||
    (!q.isFetching &&
      ((q.lastUpdated === null && q.fetchError === null) || q.didInvalidate))
  )
    return dispatch(fetchExpenses(query));

  return Promise.resolve();
};

export const deleteExpense = id => dispatch => {
  dispatch(modifyExpenseRequest(id, 'DELETE'));
  return fetch(`/api/expenses/${id}`, { method: 'DELETE' })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res;
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(modifyExpenseSuccess(id, 'DELETE', null));
    })
    .catch(e => dispatch(modifyExpenseError(id, 'DELETE', e)));
};

export const createExpense = expense => dispatch => {
  dispatch(modifyExpenseRequest(null, 'POST'));
  return fetch(`/api/expenses/`, {
    method: 'POST',
    body: JSON.stringify(expense),
  })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(modifyExpenseSuccess(res.id, 'POST', res));
    })
    .catch(e => dispatch(modifyExpenseError(null, 'POST', e)));
};

export const updateExpense = expense => dispatch => {
  dispatch(modifyExpenseRequest(expense.id, 'PUT'));
  return fetch(`/api/expenses/${expense.id}`, {
    method: 'PUT',
    body: JSON.stringify(expense),
  })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res;
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(modifyExpenseSuccess(expense.id, 'PUT', null));
    })
    .catch(e => dispatch(modifyExpenseError(expense.id, 'PUT', e)));
};
