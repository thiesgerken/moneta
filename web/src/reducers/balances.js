import { map } from 'lodash';

import {
  INVALIDATE_ALL_BALANCE_DATA,
  INVALIDATE_BALANCE_INFO,
  INVALIDATE_BALANCES,
  FETCH_BALANCES_REQUEST,
  FETCH_BALANCES_ERROR,
  FETCH_BALANCES_SUCCESS,
  FETCH_BALANCE_INFO_REQUEST,
  FETCH_BALANCE_INFO_ERROR,
  FETCH_BALANCE_INFO_SUCCESS,
  // MODIFY_BALANCE_REQUEST,
  // MODIFY_BALANCE_ERROR,
  // MODIFY_BALANCE_SUCCESS,
} from '../actions/balances';

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
    case INVALIDATE_BALANCES:
      q = { ...q, didInvalidate: true };
      break;
    case FETCH_BALANCES_REQUEST:
      q = { ...q, isFetching: true };
      break;
    case FETCH_BALANCES_SUCCESS:
      q = {
        ...q,
        isFetching: false,
        didInvalidate: false,
        data: action.data,
        lastUpdated: action.receivedAt,
        fetchError: null,
      };
      break;
    case FETCH_BALANCES_ERROR:
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

const balances = (
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
    case INVALIDATE_BALANCE_INFO:
      return { ...state, info: { ...state.info, didInvalidate: true } };
    case FETCH_BALANCE_INFO_REQUEST:
      return { ...state, info: { ...state.info, isFetching: true } };
    case FETCH_BALANCE_INFO_SUCCESS:
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
    case FETCH_BALANCE_INFO_ERROR:
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
    case FETCH_BALANCES_REQUEST:
    case FETCH_BALANCES_SUCCESS:
    case FETCH_BALANCES_ERROR:
    case INVALIDATE_BALANCES:
      return handleQueryAction(state, action);
    // TODO: on modification, we also need to invalidate all balance data.
    // case MODIFY_BALANCE_REQUEST:
    //   return { ...state, isModifying: true };
    // case MODIFY_BALANCE_SUCCESS:
    //   return {
    //     ...state,
    //     isModifying: false,
    //     didInvalidate: true,
    //     fetchError: null,
    //   };
    // case MODIFY_BALANCE_ERROR:
    //   return {
    //     ...state,
    //     isModifying: false,
    //     didInvalidate: true,
    //     fetchError: action.error,
    //   };
    case INVALIDATE_ALL_BALANCE_DATA:
      return {
        ...state,
        info: { ...state.info, didInvalidate: true },
        queries: map(state.queries, q => ({ ...q, didInvalidate: true })),
      };
    default:
      return state;
  }
};

export default balances;
