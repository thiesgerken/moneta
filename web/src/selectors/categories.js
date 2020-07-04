import { find } from 'lodash';

export const findCategory = (categories, id) => {
  if (categories.items === null || id === null) return null;
  return find(categories.items, x => x.info.id === id) || null;
};

export const findCategoryByName = (categories, name) => {
  if (categories.items === null || name === null) return null;
  return (
    find(
      categories.items,
      x => x.info.name.toUpperCase().trim() === name.toUpperCase().trim()
    ) || null
  );
};
