export const FETCH_BALANCES_REQUEST = 'FETCH_BALANCES_REQUEST';
export const fetchBalancesRequest = () => ({
  type: FETCH_BALANCES_REQUEST,
});

export const FETCH_BALANCES_SUCCESS = 'FETCH_BALANCES_SUCCESS';
export const fetchBalancesSuccess = json => ({
  type: FETCH_BALANCES_SUCCESS,
  data: json,
  receivedAt: Date.now(),
});

export const FETCH_BALANCES_ERROR = 'FETCH_BALANCES_ERROR';
export const fetchBalancesError = e => ({
  type: FETCH_BALANCES_ERROR,
  error: e,
  receivedAt: Date.now(),
});

export const INVALIDATE_BALANCES = 'INVALIDATE_BALANCES';
export const invalidateBalances = () => ({
  type: INVALIDATE_BALANCES,
});

export const MODIFY_BALANCE_REQUEST = 'MODIFY_BALANCE_REQUEST';
export const modifyBalanceRequest = (id, method) => ({
  type: MODIFY_BALANCE_REQUEST,
  id,
  method,
});

export const MODIFY_BALANCE_SUCCESS = 'MODIFY_BALANCE_SUCCESS';
export const modifyBalanceSuccess = (id, method) => ({
  type: MODIFY_BALANCE_SUCCESS,
  id,
  method,
  receivedAt: Date.now(),
});

export const MODIFY_BALANCE_ERROR = 'MODIFY_BALANCE_ERROR';
export const modifyBalanceError = (id, method, error) => ({
  type: MODIFY_BALANCE_ERROR,
  id,
  method,
  error,
  receivedAt: Date.now(),
});

export const fetchBalances = () => dispatch => {
  dispatch(fetchBalancesRequest());
  return fetch('/api/balances')
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(fetchBalancesSuccess(res));
    })
    .catch(e => dispatch(fetchBalancesError(e)));
};

export const updateBalances = () => (dispatch, getState) => {
  const { balances } = getState();

  if (
    !balances.didInvalidate &&
    balances.lastUpdated !== null &&
    Date.now() - balances.lastUpdated > 60 * 60 * 1e3
  )
    dispatch(invalidateBalances());

  if (
    !balances.isFetching &&
    ((balances.lastUpdated === null && balances.fetchError === null) ||
      balances.didInvalidate)
  )
    return dispatch(fetchBalances());

  return Promise.resolve();
};

export const deleteBalance = id => dispatch => {
  dispatch(modifyBalanceRequest(id, 'DELETE'));
  return fetch(`/api/balances/${id}`, { method: 'DELETE' })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res;
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(modifyBalanceSuccess(id, 'DELETE', null));
    })
    .catch(e => dispatch(modifyBalanceError(id, 'DELETE', e)));
};

export const createBalance = balance => dispatch => {
  dispatch(modifyBalanceRequest(null, 'POST'));
  return fetch(`/api/balances/`, {
    method: 'POST',
    body: JSON.stringify(balance),
  })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(modifyBalanceSuccess(res.id, 'POST', res));
    })
    .catch(e => dispatch(modifyBalanceError(null, 'POST', e)));
};

export const updateBalance = balance => dispatch => {
  dispatch(modifyBalanceRequest(balance.id, 'PUT'));
  return fetch(`/api/balances/${balance.id}`, {
    method: 'PUT',
    body: JSON.stringify(balance),
  })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res;
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(modifyBalanceSuccess(balance.id, 'PUT', null));
    })
    .catch(e => dispatch(modifyBalanceError(balance.id, 'PUT', e)));
};
