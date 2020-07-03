export const FETCH_USERS_REQUEST = 'FETCH_USERS_REQUEST';
export const fetchUsersRequest = () => ({
  type: FETCH_USERS_REQUEST,
});

export const FETCH_USERS_SUCCESS = 'FETCH_USERS_SUCCESS';
export const fetchUsersSuccess = json => ({
  type: FETCH_USERS_SUCCESS,
  data: json,
  receivedAt: Date.now(),
});

export const FETCH_USERS_ERROR = 'FETCH_USERS_ERROR';
export const fetchUsersError = e => ({
  type: FETCH_USERS_ERROR,
  error: e,
  receivedAt: Date.now(),
});

export const INVALIDATE_USERS = 'INVALIDATE_USERS';
export const invalidateUsers = () => ({
  type: INVALIDATE_USERS,
});

export const fetchUsers = () => dispatch => {
  dispatch(fetchUsersRequest());
  return fetch('/api/user/list')
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;
      dispatch(fetchUsersSuccess(res));
    })
    .catch(e => dispatch(fetchUsersError(e)));
};

export const updateUsers = () => (dispatch, getState) => {
  const { users } = getState();

  if (
    !users.didInvalidate &&
    users.lastUpdated !== null &&
    Date.now() - users.lastUpdated > 60 * 60 * 1e3
  )
    dispatch(invalidateUsers());

  if (
    !users.isFetching &&
    ((users.lastUpdated === null && users.fetchError === null) ||
      users.didInvalidate)
  )
    return dispatch(fetchUsers());

  return Promise.resolve();
};
