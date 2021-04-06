import { findBalanceQueryData } from '../selectors/balances';

export const FETCH_BALANCE_INFO_REQUEST = 'FETCH_BALANCE_INFO_REQUEST';
export const fetchBalanceInfoRequest = () => ({
  type: FETCH_BALANCE_INFO_REQUEST,
});

export const FETCH_BALANCE_INFO_SUCCESS = 'FETCH_BALANCE_INFO_SUCCESS';
export const fetchBalanceInfoSuccess = json => ({
  type: FETCH_BALANCE_INFO_SUCCESS,
  data: json,
  receivedAt: Date.now(),
});

export const FETCH_BALANCE_INFO_ERROR = 'FETCH_BALANCE_INFO_ERROR';
export const fetchBalanceInfoError = e => ({
  type: FETCH_BALANCE_INFO_ERROR,
  error: e,
  receivedAt: Date.now(),
});

export const FETCH_BALANCES_REQUEST = 'FETCH_BALANCES_REQUEST';
export const fetchBalancesRequest = query => ({
  type: FETCH_BALANCES_REQUEST,
  query,
});

export const FETCH_BALANCES_SUCCESS = 'FETCH_BALANCES_SUCCESS';
export const fetchBalancesSuccess = (query, json) => ({
  type: FETCH_BALANCES_SUCCESS,
  query,
  data: json,
  receivedAt: Date.now(),
});

export const FETCH_BALANCES_ERROR = 'FETCH_BALANCES_ERROR';
export const fetchBalancesError = (query, e) => ({
  type: FETCH_BALANCES_ERROR,
  error: e,
  query,
  receivedAt: Date.now(),
});

export const INVALIDATE_ALL_BALANCE_DATA = 'INVALIDATE_ALL_BALANCE_DATA';
export const invalidateAllBalanceData = () => ({
  type: INVALIDATE_ALL_BALANCE_DATA,
});

export const INVALIDATE_BALANCE_INFO = 'INVALIDATE_BALANCE_INFO';
export const invalidateBalanceInfo = () => ({
  type: INVALIDATE_BALANCE_INFO,
});

export const INVALIDATE_BALANCES = 'INVALIDATE_BALANCES';
export const invalidateBalances = query => ({
  type: INVALIDATE_BALANCES,
  query,
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

export const fetchBalances = query => dispatch => {
  dispatch(fetchBalancesRequest(query));
  return fetch('/api/balances/query', {
    method: 'POST',
    body: JSON.stringify(query),
  })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(fetchBalancesSuccess(query, res));
    })
    .catch(e => dispatch(fetchBalancesError(query, e)));
};

export const fetchBalanceInfo = () => dispatch => {
  dispatch(fetchBalanceInfoRequest());
  return fetch('/api/balances/info')
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(fetchBalanceInfoSuccess(res));
    })
    .catch(e => dispatch(fetchBalanceInfoError(e)));
};

export const updateBalanceInfo = () => (dispatch, getState) => {
  const { balances } = getState();

  if (
    !balances.info.didInvalidate &&
    balances.info.lastUpdated !== null &&
    Date.now() - balances.info.lastUpdated > 60 * 60 * 1e3
  )
    dispatch(invalidateBalanceInfo());

  if (
    !balances.info.isFetching &&
    ((balances.info.lastUpdated === null &&
      balances.info.fetchError === null) ||
      balances.info.didInvalidate)
  )
    return dispatch(fetchBalanceInfo());

  return Promise.resolve();
};

export const updateBalances = query => (dispatch, getState) => {
  const { balances } = getState();

  const q = findBalanceQueryData(balances, query);
  if (
    q !== null &&
    !q.didInvalidate &&
    q.lastUpdated !== null &&
    Date.now() - q.lastUpdated > 60 * 60 * 1e3
  )
    dispatch(invalidateBalances(query));

  if (
    q === null ||
    (!q.isFetching &&
      ((q.lastUpdated === null && q.fetchError === null) || q.didInvalidate))
  )
    return dispatch(fetchBalances(query));

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
