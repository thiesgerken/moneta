import { map } from 'lodash';

import {
  INVALIDATE_ALL_EXPENSE_DATA,
  INVALIDATE_EXPENSE_INFO,
  INVALIDATE_EXPENSES,
  FETCH_EXPENSES_REQUEST,
  FETCH_EXPENSES_ERROR,
  FETCH_EXPENSES_SUCCESS,
  FETCH_EXPENSE_INFO_REQUEST,
  FETCH_EXPENSE_INFO_ERROR,
  FETCH_EXPENSE_INFO_SUCCESS,
  // MODIFY_EXPENSE_REQUEST,
  // MODIFY_EXPENSE_ERROR,
  // MODIFY_EXPENSE_SUCCESS,
} from '../actions/expenses';

const handleQueryAction = (state, action) => {
  const { query } = action;

  // shallow copy of the state
  const newState = { ...state };

  // we are going to modify the query state, and must not change the list in `state`
  // all queries except for the one to modify can be copied by reference
  newState.queries = [];
  const qjson = JSON.stringify(query);
  let q = null;
  state.queries.forEach(x => {
    if (qjson !== JSON.stringify(x.query)) newState.queries.push(x);
    else q = x;
  });

  if (q === null) {
    q = {
      query,
      isFetching: false,
      didInvalidate: false,
      fetchError: null,
      lastUpdated: null,
      data: null,
    };
  }

  switch (action.type) {
    case INVALIDATE_EXPENSES:
      q = { ...q, didInvalidate: true };
      break;
    case FETCH_EXPENSES_REQUEST:
      q = { ...q, isFetching: true };
      break;
    case FETCH_EXPENSES_SUCCESS:
      q = {
        ...q,
        isFetching: false,
        didInvalidate: false,
        data: action.data,
        lastUpdated: action.receivedAt,
        fetchError: null,
      };
      break;
    case FETCH_EXPENSES_ERROR:
      q = {
        ...q,
        isFetching: false,
        fetchError: action.error,
        lastUpdated: action.receivedAt,
      };
      break;
    default:
  }

  newState.queries.push(q);
  return newState;
};

const expenses = (
  state = {
    info: {
      isFetching: false,
      didInvalidate: false,
      fetchError: null,
      lastUpdated: null,
      data: null,
    },
    queries: [], //  {query, isFetching, didInvalidate, fetchError, lastUpdated, data}
    isModifying: false,
    modifyError: null,
  },
  action
) => {
  switch (action.type) {
    case INVALIDATE_EXPENSE_INFO:
      return { ...state, info: { ...state.info, didInvalidate: true } };
    case FETCH_EXPENSE_INFO_REQUEST:
      return { ...state, info: { ...state.info, isFetching: true } };
    case FETCH_EXPENSE_INFO_SUCCESS:
      return {
        ...state,
        info: {
          ...state.info,
          isFetching: false,
          didInvalidate: false,
          data: action.data,
          lastUpdated: action.receivedAt,
          fetchError: null,
        },
      };
    case FETCH_EXPENSE_INFO_ERROR:
      return {
        ...state,
        info: {
          ...state.info,
          isFetching: false,
          data: [],
          fetchError: action.error,
          lastUpdated: action.receivedAt,
        },
      };
    case FETCH_EXPENSES_REQUEST:
    case FETCH_EXPENSES_SUCCESS:
    case FETCH_EXPENSES_ERROR:
    case INVALIDATE_EXPENSES:
      return handleQueryAction(state, action);
    // TODO: on modification, we also need to invalidate all expense data.
    // case MODIFY_EXPENSE_REQUEST:
    //   return { ...state, isModifying: true };
    // case MODIFY_EXPENSE_SUCCESS:
    //   return {
    //     ...state,
    //     isModifying: false,
    //     didInvalidate: true,
    //     fetchError: null,
    //   };
    // case MODIFY_EXPENSE_ERROR:
    //   return {
    //     ...state,
    //     isModifying: false,
    //     didInvalidate: true,
    //     fetchError: action.error,
    //   };
    case INVALIDATE_ALL_EXPENSE_DATA:
      return {
        ...state,
        info: { ...state.info, didInvalidate: true },
        queries: map(state.queries, q => ({ ...q, didInvalidate: true })),
      };
    default:
      return state;
  }
};

export default expenses;
