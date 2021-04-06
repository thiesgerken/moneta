import { find } from 'lodash';
import { findUserName } from './users';

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

export const displayCategoryById = (
  categories,
  users,
  userInfo,
  categoryId
) => {
  const cat = findCategory(categories, categoryId);
  if (!cat) return `[${categoryId}]`;

  if (userInfo && cat.info.userId === userInfo.id) return cat.info.name;

  const userName = findUserName(users, cat.info.userId);
  return `${userName}/${cat.info.name}`;
};

export const displayCategory = (users, userInfo, categoryId, category) => {
  if (!category) return `[${categoryId}]`;

  if (userInfo && category.info.userId === userInfo.id)
    return category.info.name;

  const userName = findUserName(users, category.info.userId);
  return `${userName}/${category.info.name}`;
};
