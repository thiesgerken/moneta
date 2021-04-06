import { connect } from 'react-redux';
import React, { useEffect, useState } from 'react';
import PropTypes from 'prop-types';
import MUIDataTable from 'mui-datatables';
import { map, forEach, filter } from 'lodash';
import {
  ThemeProvider,
  createMuiTheme,
  useTheme,
  useMediaQuery,
  Container,
  CircularProgress,
  Tooltip,
  IconButton,
  Typography,
} from '@material-ui/core';
import AddIcon from '@material-ui/icons/Add';
import { useLocation, useHistory } from 'react-router';
import green from '@material-ui/core/colors/green';
import deepOrange from '@material-ui/core/colors/deepOrange';
import * as moment from 'moment';
import tinycolor from 'tinycolor2';

import {
  updateBalances as updateBalancesAction,
  updateBalanceInfo as updateBalanceInfoAction,
} from '../actions/balances';
import { updateAccounts as updateAccountsAction } from '../actions/accounts';
import { updateUsers as updateUsersAction } from '../actions/users';
import { findBalanceQueryData } from '../selectors/balances';
import BalanceDialog from './BalanceDialog';
import { muiDatatablesLang } from '../lang';
import { findAccountByName, findAccount } from '../selectors/accounts';
import { findUserName } from '../selectors/users';

function useQuery(key) {
  const location = useLocation();
  const query = new URLSearchParams(location.search);
  const history = useHistory();

  const set = value => {
    if (value === null || value === undefined) query.delete(key);
    else query.set(key, value);

    const qs = query.toString();
    history.push(`${location.pathname}${qs.length ? '?' : ''}${qs}`);
  };

  return [query.get(key), set];
}

const Balances = ({
  balances,
  accounts,
  users,
  userInfo,
  updateBalances,
  updateBalanceInfo,
  updateAccounts,
  updateUsers,
}) => {
  const [balanceId, setBalanceId] = useQuery('id');
  const [query, setQuery] = useState([
    null,
    {
      page: 0,
      rowsPerPage: 15,
      filterBy: {},
      sortBy: [{ column: 'date', direction: 'descending' }],
      needle: null,
      from: null,
      to: null,
    },
  ]);
  const [prevQuery, currentQuery] = query;

  const currentQueryData = findBalanceQueryData(balances, currentQuery);
  const currentQueryResponse =
    currentQueryData && currentQueryData.data
      ? currentQueryData.data
      : { records: [], filteredRecordCount: 0, totalRecordCount: null };

  const isLoading = !currentQueryData || currentQueryData.isFetching;

  // let displayedQuery = currentQuery;
  let displayedResponse = currentQueryResponse;

  if (prevQuery && isLoading) {
    // continue displaying the old data for now (and a loading indicator)
    const prevQueryData = findBalanceQueryData(balances, prevQuery);
    const prevQueryResponse =
      prevQueryData && prevQueryData.data
        ? prevQueryData.data
        : { records: [], filteredRecordCount: 0, totalRecordCount: null };

    // displayedQuery = prevQuery;
    displayedResponse = prevQueryResponse;
  }

  const initialBalance = {
    id: null,
    userId: -1,
    name: '',
    iban: '',
  };

  // if (
  //   balances.items !== null &&
  //   balanceId !== null &&
  //   parseInt(balanceId, 10) >= 0
  // ) {
  //   const a = find(balances.items, x => x.id === parseInt(balanceId, 10));
  //   if (a !== undefined) initialBalance = a;
  // }

  const theme = useTheme();
  const mdUp = useMediaQuery(theme.breakpoints.up('md'));
  const xsDown = useMediaQuery(theme.breakpoints.down('xs'));

  const cellStyle = {
    textOverflow: 'ellipsis',
    overflow: 'hidden',
    display: '-webkit-box',
    WebkitLineClamp: xsDown ? 2 : 1,
    WebkitBoxOrient: 'vertical',
  };

  const myTheme = th =>
    createMuiTheme({
      ...th,
      overrides: {
        ...th.overrides,
        MUIDataTableHeadCell: {
          root: {
            '&:nth-child(1)': {
              width: '0%',
            },
            '&:nth-child(2)': {
              width: '10%',
            },
            '&:nth-child(3)': {
              width: '10%',
            },
            '&:nth-child(4)': {
              width: '10%',
            },
            '&:nth-child(5)': {
              width: '70%',
            },
          },
        },
        MUIDataTableBodyCell: {
          root: { '& div': cellStyle },
        },
        MUIDataTableSelectCell: {
          root: {
            display: 'none',
          },
        },
      },
    });

  useEffect(() => {
    updateUsers();
    updateAccounts();
    updateBalanceInfo();
    updateBalances(currentQuery);
  });

  const columns = [
    {
      name: 'id',
      label: 'ID',
      options: {
        display: false,
        filterType: 'textField',
        customFilterListOptions: { render: v => `ID: ${v}` },
      },
    },
    {
      name: 'date',
      label: 'Datum',
      options: {
        filter: false, // TODO: one could integrate from-to filtering here!
        customBodyRenderLite: i => {
          const date = moment(displayedResponse.records[i].date);

          if (xsDown) return date.format('L');
          return date.format('L LT');
        },
      },
    },
    {
      name: 'accountId',
      label: 'Konto',
      options: {
        customBodyRenderLite: i => {
          const { accountId } = displayedResponse.records[i];
          const acc = findAccount(accounts, accountId);
          if (!acc) return `[${accountId}]`;

          if (userInfo && acc.info.userId === userInfo.id) return acc.info.name;

          const userName = findUserName(users, acc.info.userId);
          return `${userName}/${acc.info.name}`;
        },
        sort: false,
        filterOptions: {
          names: map(
            userInfo
              ? filter(
                  accounts.items,
                  a => a.info.userId === userInfo.id && !a.info.hidden
                )
              : [],
            a => a.info.name
          ),
        },
        setCellProps: (_, i) => {
          const { accountId } = displayedResponse.records[i];
          const acc = findAccount(accounts, accountId);

          if (acc === null) return {};

          const { color } = acc.info;
          const style = { backgroundColor: color };

          if (
            tinycolor(color).isValid() &&
            tinycolor(color).getBrightness() <= 120
          )
            style.color = '#F7F7F7';

          return { style };
        },
      },
    },
    {
      name: 'amount',
      label: 'Betrag',
      options: {
        filterType: 'textField',
        customFilterListOptions: {
          render: v => `Betrag: ${parseFloat(v).toFixed(2)}€`,
        },
        customBodyRenderLite: i => {
          const { amount } = displayedResponse.records[i];

          let color = '';
          // eslint-disable-next-line prefer-destructuring
          if (amount < 0) color = deepOrange[800];
          // eslint-disable-next-line prefer-destructuring
          if (amount > 0) color = green[800];

          return (
            <Typography variant="inherit" style={{ color }}>
              {`${(amount / 100.0).toFixed(2)}€`}
            </Typography>
          );
        },
      },
    },
    {
      name: 'comment',
      label: 'Kommentare',
      options: {
        filter: false,
      },
    },
  ];

  const currentSortOrder = {};
  if (currentQuery.sortBy.length > 0) {
    currentSortOrder.direction =
      currentQuery.sortBy[0].direction === 'ascending' ? 'asc' : 'desc';
    currentSortOrder.name = currentQuery.sortBy[0].column;
  }

  const options = {
    selectableRows: 'single',
    selectableRowsOnClick: true,
    selectToolbarPlacement: 'none',
    enableNestedDataAccess: '.',
    serverSide: true,
    print: false,
    onTableChange: (action, tableState) => {
      const newQuery = {
        page: tableState.page,
        rowsPerPage: tableState.rowsPerPage,
        filterBy: {},
        sortBy: [],
        needle: tableState.searchText,
        from: null,
        to: null,
      };

      if (tableState.sortOrder.direction) {
        const column = tableState.sortOrder.name;

        newQuery.sortBy.push({
          column,
          direction:
            tableState.sortOrder.direction === 'asc'
              ? 'ascending'
              : 'descending',
        });
      }

      forEach(tableState.filterList, (v, i) => {
        if (v.length > 0) {
          newQuery.filterBy[columns[i].name] = map(v, x => {
            if (columns[i].name === 'accountId')
              return findAccountByName(accounts, x).info.id.toString();

            if (columns[i].name === 'amount')
              return (parseFloat(x) * 100).toFixed(0);

            if (columns[i].name.startsWith('info.is'))
              return x === 'Ja' ? 'true' : 'false';

            return x;
          });
          newQuery.filterBy[columns[i].name].sort();
        }
      });

      if (JSON.stringify(newQuery) !== JSON.stringify(currentQuery))
        setQuery([currentQuery, newQuery]);
    },
    // onRowSelectionChange: (_, rows) => {
    //   if (rows.length > 0 && rows[0].dataIndex < expenses.items.length)
    //     setExpenseId(expenses.items[rows[0].dataIndex].info.id);
    //   else setExpenseId(null);
    // },
    download: false,
    rowsPerPageOptions: mdUp ? [10, 15, 20, 50] : [15],
    rowsPerPage: currentQuery.rowsPerPage,
    page: currentQuery.page,
    count: displayedResponse.filteredRecordCount,
    sortOrder: currentSortOrder,
    filterType: 'multiselect',
    responsive: 'standard',
    customToolbar: () => (
      <>
        <Tooltip title="Neuer Kontostand">
          <IconButton onClick={() => setBalanceId(-1)}>
            <AddIcon />
          </IconButton>
        </Tooltip>
      </>
    ),
    textLabels: muiDatatablesLang,
  };
  return (
    <div style={{ margin: '-4px' }}>
      <Container maxWidth="xl" style={{ padding: '0px' }}>
        <div style={{ position: 'relative', height: '100%', width: '100%' }}>
          {isLoading && (
            <div
              style={{
                position: 'absolute',
                left: '50%',
                top: '50%',
                transform: 'translate(0%, -0%)',
                width: '0%',
                height: '0%',
                padding: '0px',
                margin: 0,
                zIndex: 9,
              }}
            >
              <CircularProgress color="secondary" />
            </div>
          )}
          <div
            style={{
              position: 'relative',
              width: '100%',
              height: '100%',
              left: '0',
              top: '0',
            }}
          >
            <ThemeProvider theme={myTheme}>
              <MUIDataTable
                title="Kontostände"
                data={displayedResponse.records}
                columns={columns}
                options={options}
              />
            </ThemeProvider>
          </div>
        </div>
      </Container>
      <BalanceDialog
        initialBalance={initialBalance}
        open={balanceId !== null && !isLoading}
        onClose={() => {
          setBalanceId(null);
        }}
      />
    </div>
  );
};

Balances.defaultProps = { userInfo: null };

Balances.propTypes = {
  updateBalances: PropTypes.func.isRequired,
  updateUsers: PropTypes.func.isRequired,
  updateBalanceInfo: PropTypes.func.isRequired,
  updateAccounts: PropTypes.func.isRequired,
  balances: PropTypes.shape({
    items: PropTypes.arrayOf(PropTypes.object),
    isFetching: PropTypes.bool,
  }).isRequired,
  accounts: PropTypes.shape({
    isFetching: PropTypes.bool.isRequired,
    items: PropTypes.arrayOf(PropTypes.object),
  }).isRequired,
  users: PropTypes.shape({
    isFetching: PropTypes.bool.isRequired,
    items: PropTypes.arrayOf(PropTypes.object),
  }).isRequired,
  userInfo: PropTypes.shape({
    id: PropTypes.number.isRequired,
  }),
};

const mapDispatchToProps = dispatch => ({
  updateBalances: query => dispatch(updateBalancesAction(query)),
  updateBalanceInfo: () => dispatch(updateBalanceInfoAction()),
  updateAccounts: () => dispatch(updateAccountsAction()),
  updateUsers: () => dispatch(updateUsersAction()),
});

const mapStateToProps = state => {
  const { balances, accounts, users, authentication } = state;
  const { userInfo } = authentication;

  return {
    balances,
    accounts,
    userInfo,
    users,
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(Balances);
