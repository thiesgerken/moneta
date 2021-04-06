import thunkMiddleware from 'redux-thunk';
import { createLogger } from 'redux-logger';
import React from 'react';
import { render } from 'react-dom';
import { createStore, applyMiddleware, compose } from 'redux';
import { Provider } from 'react-redux';
import { createBrowserHistory } from 'history';
import { ConnectedRouter, routerMiddleware } from 'connected-react-router';
import { ThemeProvider, createMuiTheme } from '@material-ui/core';
import 'typeface-oxygen';
import 'typeface-oxygen-mono';
import { LocalizationProvider } from '@material-ui/pickers';
import MomentUtils from '@date-io/moment';
import deepOrange from '@material-ui/core/colors/deepOrange';
import orange from '@material-ui/core/colors/orange';
import * as moment from 'moment';

import rootReducer from './reducers';
import App from './containers/App';
import ServiceWorker from './containers/ServiceWorker';

const history = createBrowserHistory();

const middlewares = [routerMiddleware(history), thunkMiddleware];
let composeEnhancers = compose;

if (process.env.NODE_ENV === `development`) {
  const loggerMiddleware = createLogger();
  middlewares.push(loggerMiddleware);

  composeEnhancers =
    window.__REDUX_DEVTOOLS_EXTENSION_COMPOSE__ || composeEnhancers;
}

const store = createStore(
  rootReducer(history),
  composeEnhancers(applyMiddleware(...middlewares))
);

const theme = createMuiTheme({
  typography: {
    fontFamily: ['Oxygen'].join(','),
    fontFamilyMonospaced: 'Oxygen Mono',
  },
  palette: {
    primary: orange,
    secondary: deepOrange,
  },
});

render(
  <Provider store={store}>
    <ConnectedRouter history={history}>
      <ThemeProvider theme={theme}>
        <LocalizationProvider
          dateAdapter={MomentUtils}
          dateLibInstance={moment}
        >
          <ServiceWorker />
          <App />
        </LocalizationProvider>
      </ThemeProvider>
    </ConnectedRouter>
  </Provider>,
  document.getElementById('root')
);
