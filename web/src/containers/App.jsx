import React from 'react';
import { Switch, Route } from 'react-router-dom';
import { connect } from 'react-redux';
import PropTypes from 'prop-types';

import LoginPage from './LoginPage';
import AppLayout from './AppLayout';
import {
  isAuthenticated,
  isAuthenticationInitialized,
} from '../selectors/authentication';
import { logout, checkLogin } from '../actions/authentication';
import Dashboard from './Dashboard';
import Settings from './Settings';
import Accounts from './Accounts';
import Balances from './Balances';
import Expenses from './Expenses';
import Categories from './Categories';

const App = ({ loggedIn, onLogout, initialized, onInitAuthentication }) => {
  if (!initialized) {
    onInitAuthentication();
    return <></>;
  }

  if (!loggedIn) return <LoginPage />;

  return (
    <AppLayout>
      <Switch>
        <Route path="/logout" render={onLogout} />
        <Route path="/accounts" render={() => <Accounts />} />
        <Route path="/expenses" render={() => <Expenses />} />
        <Route path="/categories" render={() => <Categories />} />
        <Route path="/balances" render={() => <Balances />} />
        <Route path="/settings" render={() => <Settings />} />
        <Route render={() => <Dashboard />} />
      </Switch>
    </AppLayout>
  );
};

App.propTypes = {
  loggedIn: PropTypes.bool.isRequired,
  initialized: PropTypes.bool.isRequired,
  onLogout: PropTypes.func.isRequired,
  onInitAuthentication: PropTypes.func.isRequired,
};

const mapStateToProps = state => {
  return {
    loggedIn: isAuthenticated(state),
    initialized: isAuthenticationInitialized(state),
  };
};

const mapDispatchToProps = dispatch => {
  return {
    onLogout: () => {
      dispatch(logout());
    },
    onInitAuthentication: () => dispatch(checkLogin()),
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(App);
