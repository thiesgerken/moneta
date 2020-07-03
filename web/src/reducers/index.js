import { combineReducers } from 'redux';
import { connectRouter } from 'connected-react-router';

import balances from './balances';
import users from './users';
import accounts from './accounts';
import categories from './categories';
import expenses from './expenses';
import authentication from './authentication';
import notifications from './notifications';

const root = history =>
  combineReducers({
    router: connectRouter(history),
    authentication,
    notifications,
    accounts,
    balances,
    categories,
    expenses,
    users,
  });

export default root;
