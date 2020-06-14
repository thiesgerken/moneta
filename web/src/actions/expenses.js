export const FETCH_EXPENSES_REQUEST = 'FETCH_EXPENSES_REQUEST';
export const fetchExpensesRequest = () => ({
  type: FETCH_EXPENSES_REQUEST,
});

export const FETCH_EXPENSES_SUCCESS = 'FETCH_EXPENSES_SUCCESS';
export const fetchExpensesSuccess = json => ({
  type: FETCH_EXPENSES_SUCCESS,
  data: json,
  receivedAt: Date.now(),
});

export const FETCH_EXPENSES_ERROR = 'FETCH_EXPENSES_ERROR';
export const fetchExpensesError = e => ({
  type: FETCH_EXPENSES_ERROR,
  error: e,
  receivedAt: Date.now(),
});

export const INVALIDATE_EXPENSES = 'INVALIDATE_EXPENSES';
export const invalidateExpenses = () => ({
  type: INVALIDATE_EXPENSES,
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

export const fetchExpenses = () => dispatch => {
  dispatch(fetchExpensesRequest());
  return fetch('/api/expenses')
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(fetchExpensesSuccess(res));
    })
    .catch(e => dispatch(fetchExpensesError(e)));
};

export const updateExpenses = () => (dispatch, getState) => {
  const { expenses } = getState();

  if (
    !expenses.didInvalidate &&
    expenses.lastUpdated !== null &&
    Date.now() - expenses.lastUpdated > 60 * 60 * 1e3
  )
    dispatch(invalidateExpenses());

  if (
    !expenses.isFetching &&
    ((expenses.lastUpdated === null && expenses.fetchError === null) ||
      expenses.didInvalidate)
  )
    return dispatch(fetchExpenses());

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
