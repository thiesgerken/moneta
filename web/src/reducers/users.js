import {
  INVALIDATE_USERS,
  FETCH_USERS_REQUEST,
  FETCH_USERS_ERROR,
  FETCH_USERS_SUCCESS,
} from '../actions/users';

const users = (
  state = {
    isFetching: false,
    didInvalidate: false,
    items: [],
    fetchError: null,
    lastUpdated: null,
  },
  action
) => {
  switch (action.type) {
    case INVALIDATE_USERS:
      return { ...state, didInvalidate: true };
    case FETCH_USERS_REQUEST:
      return { ...state, isFetching: true };
    case FETCH_USERS_SUCCESS:
      return {
        ...state,
        isFetching: false,
        didInvalidate: false,
        items: action.data,
        lastUpdated: action.receivedAt,
        fetchError: null,
      };
    case FETCH_USERS_ERROR:
      return {
        ...state,
        isFetching: false,
        items: [],
        fetchError: action.error,
        lastUpdated: action.receivedAt,
      };
    default:
      return state;
  }
};

export default users;
