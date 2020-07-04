import {
  INVALIDATE_BALANCES,
  FETCH_BALANCES_REQUEST,
  FETCH_BALANCES_ERROR,
  FETCH_BALANCES_SUCCESS,
  MODIFY_BALANCE_REQUEST,
  MODIFY_BALANCE_ERROR,
  MODIFY_BALANCE_SUCCESS,
} from '../actions/balances';

const balances = (
  state = {
    isFetching: false,
    didInvalidate: false,
    items: [],
    fetchError: null,
    lastUpdated: null,
    isModifying: false,
    modifyError: null,
  },
  action
) => {
  switch (action.type) {
    case INVALIDATE_BALANCES:
      return { ...state, didInvalidate: true };
    case FETCH_BALANCES_REQUEST:
      return { ...state, isFetching: true };
    case FETCH_BALANCES_SUCCESS:
      return {
        ...state,
        isFetching: false,
        didInvalidate: false,
        items: action.data,
        lastUpdated: action.receivedAt,
        fetchError: null,
      };
    case FETCH_BALANCES_ERROR:
      return {
        ...state,
        isFetching: false,
        items: [],
        fetchError: action.error,
        lastUpdated: action.receivedAt,
      };
    case MODIFY_BALANCE_REQUEST:
      return { ...state, isModifying: true };
    case MODIFY_BALANCE_SUCCESS:
      return {
        ...state,
        isModifying: false,
        didInvalidate: true,
        fetchError: null,
      };
    case MODIFY_BALANCE_ERROR:
      return {
        ...state,
        isModifying: false,
        didInvalidate: true,
        fetchError: action.error,
      };
    default:
      return state;
  }
};

export default balances;
