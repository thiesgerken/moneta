import { find } from 'lodash';

export const findUser = (users, id) => {
  if (users.items === null || id === null) return null;
  return find(users.items, x => x.id === id) || null;
};

export const findUserName = (users, id) => {
  const user = findUser(users, id);
  if (user === null) return `${id}`;

  return user.fullName.split(' ')[0];
};

export const findUserByName = (users, name) => {
  if (users.items === null || name === null) return null;
  return (
    find(
      users.items,
      x => x.name.toUpperCase().trim() === name.toUpperCase().trim()
    ) || null
  );
};
