import { connect } from 'react-redux';
import React, { useEffect, useState } from 'react';
import PropTypes from 'prop-types';
import MUIDataTable from 'mui-datatables';
import {
  ThemeProvider,
  createMuiTheme,
  useTheme,
  useMediaQuery,
  Container,
  CircularProgress,
  Tooltip,
  IconButton,
} from '@material-ui/core';
import AddIcon from '@material-ui/icons/Add';
import { useLocation, useHistory } from 'react-router';
import * as moment from 'moment';
import { forEach, map, sortBy, uniqBy, filter } from 'lodash';
import tinycolor from 'tinycolor2';

import {
  updateExpenses as updateExpensesAction,
  updateExpenseInfo as updateExpenseInfoAction,
} from '../actions/expenses';
import { updateAccounts as updateAccountsAction } from '../actions/accounts';
import { updateUsers as updateUsersAction } from '../actions/users';
import { updateCategories as updateCategoriesAction } from '../actions/categories';
import ExpenseDialog from './ExpenseDialog';
import { muiDatatablesLang } from '../lang';
import { findExpenseQueryData } from '../selectors/expenses';
import { findAccount, findAccountByName } from '../selectors/accounts';
import { findCategory, findCategoryByName } from '../selectors/categories';
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

const cellStyle = {
  textOverflow: 'ellipsis',
  overflow: 'hidden',
  display: '-webkit-box',
  WebkitLineClamp: 1,
  WebkitBoxOrient: 'vertical',
};

const Expenses = ({
  users,
  updateUsers,
  expenses,
  updateExpenses,
  updateExpenseInfo,
  accounts,
  updateAccounts,
  categories,
  updateCategories,
  userInfo,
}) => {
  const [expenseId, setExpenseId] = useQuery('id');
  const [query, setQuery] = useState([
    null,
    {
      page: 0,
      rowsPerPage: 15,
      filterBy: {},
      sortBy: [{ column: 'info.bookingEnd', direction: 'descending' }],
      needle: null,
      from: null,
      to: null,
    },
  ]);
  const [prevQuery, currentQuery] = query;

  const currentQueryData = findExpenseQueryData(expenses, currentQuery);
  const currentQueryResponse =
    currentQueryData && currentQueryData.data
      ? currentQueryData.data
      : { records: [], filteredRecordCount: 0, totalRecordCount: null };

  const isLoading = !currentQueryData || currentQueryData.isFetching;
  // let displayedQuery = currentQuery;
  let displayedResponse = currentQueryResponse;

  if (prevQuery && isLoading) {
    // continue displaying the old data for now (and a loading indicator)
    const prevQueryData = findExpenseQueryData(expenses, prevQuery);
    const prevQueryResponse =
      prevQueryData && prevQueryData.data
        ? prevQueryData.data
        : { records: [], filteredRecordCount: 0, totalRecordCount: null };

    // displayedQuery = prevQuery;
    displayedResponse = prevQueryResponse;
  }

  const initialExpense = {
    id: null,
    userId: -1,
    name: '',
    iban: '',
  };

  // if (
  //   expenses.items !== null &&
  //   expenseId !== null &&
  //   parseInt(expenseId, 10) >= 0
  // ) {
  //   const a = find(expenses.items, x => x.id === parseInt(expenseId, 10));
  //   if (a !== undefined) initialExpense = a;
  // }

  const theme = useTheme();
  const mdUp = useMediaQuery(theme.breakpoints.up('md'));
  const xsDown = useMediaQuery(theme.breakpoints.down('xs'));

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
              width: '15%',
            },
            '&:nth-child(4)': {
              width: '12%',
            },
            '&:nth-child(5)': {
              width: '15%',
            },
            '&:nth-child(6)': {
              width: '8%',
            },
            '&:nth-child(7)': {
              width: '10%',
            },
            '&:nth-child(8)': {
              width: '20%',
            },
            '&:nth-child(9)': {
              width: '10%',
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
    updateCategories();
    updateExpenseInfo();
    updateExpenses(currentQuery);
  });

  // TODO: how to control column widths?
  // TODO: add ellipses in text fields

  const columns = [
    {
      name: 'info.id',
      label: 'ID',
      options: {
        display: false,
        filterType: 'textField',
        customFilterListOptions: { render: v => `ID: ${v}` },
      },
    },
    {
      name: 'info.bookingEnd',
      label: mdUp ? 'Buchungszeitraum' : 'Datum',
      options: {
        filter: false, // TODO: one could integrate from-to filtering here!
        customBodyRenderLite: i => {
          const r = displayedResponse.records[i];
          const st = moment(r.info.bookingStart);
          const en = moment(r.info.bookingEnd);

          if (xsDown) return st.format('L');

          if (st.isSame(en)) return st.format('L LT');
          if (st.isSame(en, 'day'))
            return `${st.format('L LT')} - ${en.format('LT')}`;
          return `${st.format('L')} - ${en.format('L')}`;
        },
      },
    },
    {
      name: 'info.title',
      label: 'Titel',
      options: {
        filter: false,
      },
    },
    {
      name: 'categories',
      label: 'Kategorien',
      options: {
        customBodyRenderLite: i => {
          const cs = uniqBy(
            sortBy(displayedResponse.records[i].categories, [
              c => -1 * c.weight,
            ]),
            t => t.categoryId
          );

          return (
            <div style={{ margin: theme.spacing(2) }}>
              {map(cs, c => {
                const cat = findCategory(categories, c.categoryId);
                if (!cat) return `[${c.categoryId}]`;

                if (userInfo && cat.info.userId === userInfo.id)
                  return cat.info.name;

                const userName = findUserName(users, cat.info.userId);
                return `${userName}/${cat.info.name}`;
              }).join(', ')}
            </div>
          );
        },
        filterOptions: {
          names: map(
            userInfo
              ? filter(categories.items, c => c.info.userId === userInfo.id)
              : [],
            c => c.info.name
          ),
        },
        sort: false,
        setCellProps: (_, i) => {
          const data = map(displayedResponse.records[i].categories, x => ({
            ...x,
            color:
              findCategory(categories, x.categoryId)?.info?.color ||
              '#00FFFFFF',
          }));

          const style = { padding: 0 };
          if (data.length === 0) return { style };

          if (
            tinycolor(data[0].color).isValid() &&
            tinycolor(data[0].color).getBrightness() <= 120
          )
            style.color = '#F7F7F7';

          if (data.length === 1) {
            style.backgroundColor = data[0].color;
          } else {
            const colors = [];
            const weights = [];
            const sums = [0];

            data.forEach(d => {
              const w = Math.abs(d.amount);
              colors.push(d.color);
              weights.push(w);
              sums.push(sums[sums.length - 1] + w);
            });

            const totalWeight = weights.reduce((a, b) => a + b, 0);
            const segments = [];

            for (let j = 0; j < colors.length; j += 1) {
              let s = colors[j];
              if (j !== 0)
                s += ` ${((sums[j] / totalWeight) * 100).toFixed(0)}%`;
              if (j !== colors.length - 1) {
                s += `, ${colors[j]} ${(
                  (sums[j + 1] / totalWeight) *
                  100
                ).toFixed(0)}%`;
              }
              segments.push(s);
            }

            style.backgroundImage = `linear-gradient(to right, ${segments.join(
              ', '
            )})`;
          }
          return { style };
        },
      },
    },
    {
      name: 'transactions',
      label: 'Konten',
      options: {
        customBodyRenderLite: i => {
          return (
            <div style={{ margin: theme.spacing(2) }}>
              {map(
                displayedResponse.records[i].meta.accountsByAmount,
                ({ accountId }) => {
                  const acc = findAccount(accounts, accountId);
                  if (!acc) return `[${accountId}]`;

                  if (userInfo && acc.info.userId === userInfo.id)
                    return acc.info.name;

                  const userName = findUserName(users, acc.info.userId);
                  return `${userName}/${acc.info.name}`;
                }
              ).join(', ')}
            </div>
          );
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
          const data = map(
            displayedResponse.records[i].meta.accountsByAmount,
            x => ({
              ...x,
              color:
                findAccount(accounts, x.accountId)?.info?.color || '#00FFFFFF',
            })
          );

          const style = { padding: 0 };
          if (data.length === 0) return { style };

          if (
            tinycolor(data[0].color).isValid() &&
            tinycolor(data[0].color).getBrightness() <= 120
          )
            style.color = '#F7F7F7';

          if (data.length === 1) {
            style.backgroundColor = data[0].color;
          } else {
            const colors = [];
            const weights = [];
            const sums = [0];

            data.forEach(d => {
              const w = Math.abs(d.amount);
              colors.push(d.color);
              weights.push(w);
              sums.push(sums[sums.length - 1] + w);
            });

            const totalWeight = weights.reduce((a, b) => a + b, 0);
            const segments = [];
            for (let j = 0; j < colors.length; j += 1) {
              let s = colors[j];
              if (j !== 0)
                s += ` ${((sums[j] / totalWeight) * 100).toFixed(0)}%`;
              if (j !== colors.length - 1) {
                s += `, ${colors[j]} ${(
                  (sums[j + 1] / totalWeight) *
                  100
                ).toFixed(0)}%`;
              }
              segments.push(s);
            }

            style.backgroundImage = `linear-gradient(to right, ${segments.join(
              ', '
            )})`;
          }
          return { style };
        },
      },
    },
    {
      name: 'totalAmount',
      label: 'Betrag',
      options: {
        // TODO: highlight amount depending on sign
        customBodyRenderLite: i =>
          `${(displayedResponse.records[i].totalAmount / 100.0).toFixed(2)}€`,
        filter: false,
        sort: false,
      },
    },
    {
      name: 'info.store',
      label: 'Geschäft',
      options: {
        display: mdUp,
        filter: true,
        filterOptions: {
          names: expenses.info.data ? expenses.info.data.filterHints.store : [],
        },
      },
    },
    {
      name: 'info.description',
      label: 'Beschreibung',
      options: {
        display: mdUp,
        filter: false,
      },
    },
    {
      name: 'info.comments',
      label: 'Kommentare',
      options: {
        display: mdUp,
        filter: false,
      },
    },
    {
      name: 'info.isPreliminary',
      label: 'Vorläufig',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => 'Vorläufig' },
      },
    },
    {
      name: 'info.isTaxRelevant',
      label: 'Steuerrelevant',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => 'Steuerrelevant' },
      },
    },
    {
      name: 'info.isTemplate',
      label: 'Vorlage',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => 'Vorlage' },
      },
    },
    {
      name: 'info.isUnchecked',
      label: 'Unüberprüft',
      options: {
        display: false,
        filterType: 'checkbox',
        customFilterListOptions: { render: () => `nicht überprüft` },
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
            // flags
            if (typeof x === 'boolean') return (!x).toString();

            if (columns[i].name === 'transactions')
              return findAccountByName(accounts, x).info.id.toString();

            if (columns[i].name === 'categories')
              return findCategoryByName(categories, x).info.id.toString();

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
        <Tooltip title="Neue Ausgabe">
          <IconButton onClick={() => setExpenseId(-1)}>
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
                title="Ausgaben"
                data={displayedResponse.records}
                columns={columns}
                options={options}
              />
            </ThemeProvider>
          </div>
        </div>
      </Container>
      <ExpenseDialog
        initialExpense={initialExpense}
        open={expenseId !== null && !isLoading}
        onClose={() => {
          setExpenseId(null);
        }}
      />
    </div>
  );
};

Expenses.defaultProps = { userInfo: null };

Expenses.propTypes = {
  updateExpenses: PropTypes.func.isRequired,
  updateUsers: PropTypes.func.isRequired,
  updateExpenseInfo: PropTypes.func.isRequired,
  updateCategories: PropTypes.func.isRequired,
  updateAccounts: PropTypes.func.isRequired,
  expenses: PropTypes.shape({
    info: PropTypes.shape({
      isFetching: PropTypes.bool.isRequired,
      data: PropTypes.shape({
        filterHints: PropTypes.shape({
          store: PropTypes.arrayOf(PropTypes.string),
        }),
      }),
    }).isRequired,
    queries: PropTypes.arrayOf(
      PropTypes.shape({
        isFetching: PropTypes.bool.isRequired,
        // eslint-disable-next-line react/forbid-prop-types
        data: PropTypes.object,
        // eslint-disable-next-line react/forbid-prop-types
        query: PropTypes.object.isRequired,
      })
    ),
  }).isRequired,
  accounts: PropTypes.shape({
    isFetching: PropTypes.bool.isRequired,
    items: PropTypes.arrayOf(PropTypes.object),
  }).isRequired,
  users: PropTypes.shape({
    isFetching: PropTypes.bool.isRequired,
    items: PropTypes.arrayOf(PropTypes.object),
  }).isRequired,
  categories: PropTypes.shape({
    isFetching: PropTypes.bool.isRequired,
    items: PropTypes.arrayOf(PropTypes.object),
  }).isRequired,
  userInfo: PropTypes.shape({
    id: PropTypes.number.isRequired,
  }),
};

const mapDispatchToProps = dispatch => ({
  updateExpenses: query => dispatch(updateExpensesAction(query)),
  updateExpenseInfo: () => dispatch(updateExpenseInfoAction()),
  updateAccounts: () => dispatch(updateAccountsAction()),
  updateCategories: () => dispatch(updateCategoriesAction()),
  updateUsers: () => dispatch(updateUsersAction()),
});

const mapStateToProps = state => {
  const { expenses, accounts, categories, authentication, users } = state;
  const { userInfo } = authentication;

  return {
    expenses,
    accounts,
    categories,
    userInfo,
    users,
  };
};

export default connect(mapStateToProps, mapDispatchToProps)(Expenses);
