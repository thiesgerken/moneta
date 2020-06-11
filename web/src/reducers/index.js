import { combineReducers } from 'redux';
import { connectRouter } from 'connected-react-router';

import balances from './balances';
import accounts from './accounts';
import authentication from './authentication';
import notifications from './notifications';

const root = history =>
  combineReducers({
    router: connectRouter(history),
    authentication,
    notifications,
    accounts,
    balances,
  });

export default root;
