import {
  INVALIDATE_CATEGORIES,
  FETCH_CATEGORIES_REQUEST,
  FETCH_CATEGORIES_ERROR,
  FETCH_CATEGORIES_SUCCESS,
  MODIFY_CATEGORY_REQUEST,
  MODIFY_CATEGORY_ERROR,
  MODIFY_CATEGORY_SUCCESS,
} from '../actions/categories';

const categories = (
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
    case INVALIDATE_CATEGORIES:
      return { ...state, didInvalidate: true };
    case FETCH_CATEGORIES_REQUEST:
      return { ...state, isFetching: true };
    case FETCH_CATEGORIES_SUCCESS:
      return {
        ...state,
        isFetching: false,
        didInvalidate: false,
        items: action.data,
        lastUpdated: action.receivedAt,
        fetchError: null,
      };
    case FETCH_CATEGORIES_ERROR:
      return {
        ...state,
        isFetching: false,
        items: [],
        fetchError: action.error,
        lastUpdated: action.receivedAt,
      };
    case MODIFY_CATEGORY_REQUEST:
      return { ...state, isModifying: true };
    case MODIFY_CATEGORY_SUCCESS:
      return {
        ...state,
        isModifying: false,
        didInvalidate: true,
        fetchError: null,
      };
    case MODIFY_CATEGORY_ERROR:
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

export default categories;
