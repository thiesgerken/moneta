import {
  INVALIDATE_EXPENSES,
  FETCH_EXPENSES_REQUEST,
  FETCH_EXPENSES_ERROR,
  FETCH_EXPENSES_SUCCESS,
  MODIFY_EXPENSE_REQUEST,
  MODIFY_EXPENSE_ERROR,
  MODIFY_EXPENSE_SUCCESS,
} from '../actions/expenses';

const expenses = (
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
    case INVALIDATE_EXPENSES:
      return { ...state, didInvalidate: true };
    case FETCH_EXPENSES_REQUEST:
      return { ...state, isFetching: true };
    case FETCH_EXPENSES_SUCCESS:
      return {
        ...state,
        isFetching: false,
        didInvalidate: false,
        items: action.data,
        lastUpdated: action.receivedAt,
        fetchError: null,
      };
    case FETCH_EXPENSES_ERROR:
      return {
        ...state,
        isFetching: false,
        items: [],
        fetchError: action.error,
        lastUpdated: action.receivedAt,
      };
    case MODIFY_EXPENSE_REQUEST:
      return { ...state, isModifying: true };
    case MODIFY_EXPENSE_SUCCESS:
      return {
        ...state,
        isModifying: false,
        didInvalidate: true,
        fetchError: null,
      };
    case MODIFY_EXPENSE_ERROR:
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

export default expenses;
