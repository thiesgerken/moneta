export const FETCH_CATEGORIES_REQUEST = 'FETCH_CATEGORIES_REQUEST';
export const fetchCategoriesRequest = () => ({
  type: FETCH_CATEGORIES_REQUEST,
});

export const FETCH_CATEGORIES_SUCCESS = 'FETCH_CATEGORIES_SUCCESS';
export const fetchCategoriesSuccess = json => ({
  type: FETCH_CATEGORIES_SUCCESS,
  data: json,
  receivedAt: Date.now(),
});

export const FETCH_CATEGORIES_ERROR = 'FETCH_CATEGORIES_ERROR';
export const fetchCategoriesError = e => ({
  type: FETCH_CATEGORIES_ERROR,
  error: e,
  receivedAt: Date.now(),
});

export const INVALIDATE_CATEGORIES = 'INVALIDATE_CATEGORIES';
export const invalidateCategories = () => ({
  type: INVALIDATE_CATEGORIES,
});

export const MODIFY_CATEGORY_REQUEST = 'MODIFY_CATEGORY_REQUEST';
export const modifyCategoryRequest = (id, method) => ({
  type: MODIFY_CATEGORY_REQUEST,
  id,
  method,
});

export const MODIFY_CATEGORY_SUCCESS = 'MODIFY_CATEGORY_SUCCESS';
export const modifyCategoriesuccess = (id, method) => ({
  type: MODIFY_CATEGORY_SUCCESS,
  id,
  method,
  receivedAt: Date.now(),
});

export const MODIFY_CATEGORY_ERROR = 'MODIFY_CATEGORY_ERROR';
export const modifyCategoryError = (id, method, error) => ({
  type: MODIFY_CATEGORY_ERROR,
  id,
  method,
  error,
  receivedAt: Date.now(),
});

export const fetchCategories = () => dispatch => {
  dispatch(fetchCategoriesRequest());
  return fetch('/api/categories')
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(fetchCategoriesSuccess(res));
    })
    .catch(e => dispatch(fetchCategoriesError(e)));
};

export const updateCategories = () => (dispatch, getState) => {
  const { categories } = getState();

  if (
    !categories.didInvalidate &&
    categories.lastUpdated !== null &&
    Date.now() - categories.lastUpdated > 60 * 60 * 1e3
  )
    dispatch(invalidateCategories());

  if (
    !categories.isFetching &&
    ((categories.lastUpdated === null && categories.fetchError === null) ||
      categories.didInvalidate)
  )
    return dispatch(fetchCategories());

  return Promise.resolve();
};

export const deleteCategory = id => dispatch => {
  dispatch(modifyCategoryRequest(id, 'DELETE'));
  return fetch(`/api/categories/${id}`, { method: 'DELETE' })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res;
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(modifyCategoriesuccess(id, 'DELETE', null));
    })
    .catch(e => dispatch(modifyCategoryError(id, 'DELETE', e)));
};

export const createCategory = category => dispatch => {
  dispatch(modifyCategoryRequest(null, 'POST'));
  return fetch(`/api/categories/`, {
    method: 'POST',
    body: JSON.stringify(category),
  })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res.json();
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(modifyCategoriesuccess(res.id, 'POST', res));
    })
    .catch(e => dispatch(modifyCategoryError(null, 'POST', e)));
};

export const updateCategory = category => dispatch => {
  dispatch(modifyCategoryRequest(category.id, 'PUT'));
  return fetch(`/api/categories/${category.id}`, {
    method: 'PUT',
    body: JSON.stringify(category),
  })
    .then(res => {
      if (!res.ok) throw Error(`${res.status} ${res.statusText}`);
      return res;
    })
    .then(res => {
      if (res.error) throw res.error;

      dispatch(modifyCategoriesuccess(category.id, 'PUT', null));
    })
    .catch(e => dispatch(modifyCategoryError(category.id, 'PUT', e)));
};
